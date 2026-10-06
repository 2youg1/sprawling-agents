// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Owned Windows native execution. The launch ordering and boundary contract
//! are specified by `crates/runtime/spec/Tools/Exec/NativeWindows.lean`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::num::{NonZeroU16, NonZeroUsize};
use std::path::{Path, PathBuf};

mod packet;

/// Inputs already admitted by the runtime; the directory is a disposable copy.
#[derive(Debug)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub directory: PathBuf,
    pub environment: BTreeMap<OsString, OsString>,
    /// The run Job is owned by the backlog; assignment occurs before resume.
    pub parent_job: NonZeroUsize,
    /// Hundredths of a percent of the machine's CPU, from 1 to 10,000.
    pub cpu_rate: NonZeroU16,
    /// A unique, previously unused AppContainer profile name for this command.
    pub profile: String,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Encode,
    Launch,
    Poll,
    Terminate,
    Cleanup,
}

/// The failed action and Windows/HRESULT code are preserved without fallback.
#[derive(Debug)]
pub struct Failure {
    pub action: Action,
    pub code: u32,
    pub phase: String,
    pub cleanup_code: Option<u32>,
    /// A failed launch still owns resources whose cleanup the OS refused.
    pub resources: Option<Box<OwnedProcess>>,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Windows native confinement: {:?} at {}, code {}, cleanup {:?}; {}",
            self.action,
            self.phase,
            self.code,
            self.cleanup_code,
            self.recovery()
        )
    }
}
impl std::error::Error for Failure {}

impl Failure {
    pub const fn recovery(&self) -> &'static str {
        "check Windows AppContainer and Job support and disposable directory permissions; retry the sandbox command"
    }
}

#[repr(C)]
#[derive(Debug, Default)]
struct Record {
    cpu: usize,
    job: usize,
    process: usize,
    pid: usize,
    finished: usize,
    exit: usize,
    cleanup: usize,
    cleanup_error: usize,
    parent_job: usize,
    run_assigned: usize,
    command_assigned: usize,
    identity_verified: usize,
    root_security: usize,
    profile_created: usize,
    failure_phase: [u8; 32],
}

#[expect(unsafe_code, reason = "native confinement leaf declarations")]
unsafe extern "C" {
    fn sprawling_native_launch(
        text: *const u16,
        units: usize,
        record: *mut Record,
        bytes: usize,
    ) -> u32;
    fn sprawling_native_poll(record: *mut Record, bytes: usize) -> u32;
    fn sprawling_native_terminate(record: *mut Record, bytes: usize) -> u32;
    fn sprawling_native_cleanup(
        record: *mut Record,
        bytes: usize,
        profile: *const u16,
        units: usize,
    ) -> u32;
    #[cfg(test)]
    fn sprawling_native_may_resume(record: *const Record, bytes: usize) -> u32;
    fn sprawling_native_packet_valid(text: *const u16, units: usize) -> u32;
}

/// Owns both the Job and process handles and the per-command profile.
/// Call `close` to observe cleanup failures; Drop is the emergency teardown
/// after a caller abandons the command and reports any cleanup error to stderr.
#[derive(Debug)]
pub struct OwnedProcess {
    record: Record,
    context: Vec<u16>,
    pid: u32,
}

/// Reads the Windows installation root through the existing safe SDK adapter.
/// A failed OS query remains an observed native launch refusal.
pub fn windows_root() -> Result<PathBuf, Failure> {
    let system = PathBuf::from(winsafe::GetSystemDirectory().map_err(|error| Failure {
        action: Action::Launch,
        code: error.raw(),
        phase: "Windows root".to_owned(),
        cleanup_code: None,
        resources: None,
    })?);
    system
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Failure {
            action: Action::Launch,
            code: 87,
            phase: "Windows root has no parent".to_owned(),
            cleanup_code: None,
            resources: None,
        })
}

pub fn launch(launch: &Launch) -> Result<OwnedProcess, Failure> {
    let text = packet::encode(launch)?;
    let mut context = packet::terminated(launch.directory.as_os_str())?;
    context.extend(packet::terminated(launch.profile.as_ref())?);
    // SAFETY: text is a live UTF16 buffer with the exact length passed, and
    // packet validation borrows it without retaining pointers or changing OS state.
    #[expect(
        unsafe_code,
        reason = "validate the foreign packet before native effects"
    )]
    let valid = unsafe { sprawling_native_packet_valid(text.as_ptr(), text.len()) };
    if valid == 0 {
        return Err(Failure {
            action: Action::Encode,
            code: 87,
            phase: String::new(),
            cleanup_code: None,
            resources: None,
        });
    }
    let mut record = Record {
        cpu: usize::from(launch.cpu_rate.get()),
        parent_job: launch.parent_job.get(),
        ..Record::default()
    };
    // SAFETY: text and record are live, exclusively borrowed buffers with the
    // exact lengths passed; the leaf retains owned OS allocations and handles, no borrowed pointers.
    #[expect(unsafe_code, reason = "the confined Windows launch leaf")]
    let code = unsafe {
        sprawling_native_launch(
            text.as_ptr(),
            text.len(),
            &raw mut record,
            std::mem::size_of::<Record>(),
        )
    };
    if code != 0 {
        let cleanup_code = match record.cleanup_error {
            0 => None,
            value => Some(u32::try_from(value).map_err(|_| Failure {
                action: Action::Cleanup,
                code: 87,
                phase: String::new(),
                cleanup_code: None,
                resources: None,
            })?),
        };
        let phase = record
            .failure_phase
            .iter()
            .copied()
            .take_while(|unit| *unit != 0)
            .collect::<Vec<_>>();
        let resources = if record.job != 0
            || record.process != 0
            || record.root_security != 0
            || record.profile_created != 0
        {
            Some(Box::new(OwnedProcess {
                record,
                context,
                pid: 0,
            }))
        } else {
            None
        };
        return Err(Failure {
            action: Action::Launch,
            code,
            phase: String::from_utf8_lossy(&phase).into_owned(),
            cleanup_code,
            resources,
        });
    }
    let mut process = OwnedProcess {
        record,
        context,
        pid: 0,
    };
    process.pid = u32::try_from(process.record.pid).map_err(|_| Failure {
        action: Action::Launch,
        code: 87,
        phase: String::new(),
        cleanup_code: None,
        resources: None,
    })?;
    Ok(process)
}

impl OwnedProcess {
    pub fn id(&self) -> u32 {
        self.pid
    }

    pub fn raw_process_handle(&self) -> std::os::windows::io::RawHandle {
        std::ptr::without_provenance_mut(self.record.process)
    }

    /// The root's exit is returned only after its remaining descendants end.
    pub fn try_wait(&mut self) -> Result<Option<u32>, Failure> {
        if self.record.finished == 0 {
            // SAFETY: this exclusively owned record contains handles created by
            // launch and kept open until cleanup; its byte length is exact.
            #[expect(unsafe_code, reason = "poll the owned native process")]
            let code = unsafe {
                sprawling_native_poll(&raw mut self.record, std::mem::size_of::<Record>())
            };
            checked(Action::Poll, code)?;
        }
        if self.record.finished == 0 {
            return Ok(None);
        }
        self.cleanup()?;
        u32::try_from(self.record.exit)
            .map(Some)
            .map_err(|_| Failure {
                action: Action::Poll,
                code: 87,
                phase: String::new(),
                cleanup_code: None,
                resources: None,
            })
    }

    pub fn kill(&mut self) -> Result<(), Failure> {
        // SAFETY: the exclusively borrowed record owns its still-open Job.
        #[expect(unsafe_code, reason = "terminate the confined process tree")]
        let code = unsafe {
            sprawling_native_terminate(&raw mut self.record, std::mem::size_of::<Record>())
        };
        checked(Action::Terminate, code)
    }

    pub fn close(mut self) -> Result<(), Failure> {
        if let Err(mut failure) = self.cleanup() {
            failure.resources = Some(Box::new(self));
            return Err(failure);
        }
        Ok(())
    }

    pub fn retry_cleanup(&mut self) -> Result<(), Failure> {
        self.cleanup()
    }

    fn cleanup(&mut self) -> Result<(), Failure> {
        if self.record.cleanup != 0 {
            return Ok(());
        }
        // SAFETY: record is exclusively owned and profile is a live terminated
        // UTF16 buffer; the leaf closes each handle once and clears its slot.
        #[expect(unsafe_code, reason = "release native command resources")]
        let code = unsafe {
            sprawling_native_cleanup(
                &raw mut self.record,
                std::mem::size_of::<Record>(),
                self.context.as_ptr(),
                self.context.len(),
            )
        };
        checked(Action::Cleanup, code)
    }
}

impl Drop for OwnedProcess {
    fn drop(&mut self) {
        if let Err(failure) = self.cleanup() {
            eprintln!(
                "Windows native confinement: {:?} at {}, code {}, cleanup {:?}; {}",
                failure.action,
                failure.phase,
                failure.code,
                failure.cleanup_code,
                failure.recovery()
            );
        }
    }
}

fn checked(action: Action, code: u32) -> Result<(), Failure> {
    if code == 0 {
        Ok(())
    } else {
        Err(Failure {
            action,
            code,
            phase: String::new(),
            cleanup_code: None,
            resources: None,
        })
    }
}

#[cfg(test)]
#[allow(
    unsafe_code,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn native_resume_guard_matches_every_readiness_state(
            run in prop_oneof![Just(0_usize), Just(1_usize), any::<usize>()],
            command in prop_oneof![Just(0_usize), Just(1_usize), any::<usize>()],
            identity in prop_oneof![Just(0_usize), Just(1_usize), any::<usize>()],
        ) {
            let record = Record { run_assigned: run, command_assigned: command, identity_verified: identity, ..Record::default() };
            // SAFETY: the fully initialized record is lent with its exact size.
            let found = unsafe { sprawling_native_may_resume(&raw const record, std::mem::size_of::<Record>()) };
            prop_assert_eq!(found != 0, run == 1 && command == 1 && identity == 1);
        }
    }

    proptest! {
        #[test]
        fn native_packets_accept_real_fields_and_reject_corruption(
            fields in proptest::collection::vec(proptest::collection::vec(1_u16..=u16::MAX, 1..64), 6),
            key in "[A-Za-z_][A-Za-z0-9_]{0,24}",
            value in proptest::collection::vec(1_u16..=u16::MAX, 0..64),
            offset in 0_usize..512,
            replacement in any::<u16>(),
        ) {
            let mut units = Vec::new();
            for field in fields { units.extend(field); units.push(0); }
            units.extend(key.encode_utf16()); units.push(u16::from(b'='));
            units.extend(value); units.extend([0, 0]);
            // SAFETY: units is the entire live buffer borrowed by the parser.
            prop_assert_eq!(unsafe { sprawling_native_packet_valid(units.as_ptr(), units.len()) }, 1);
            prop_assert!(packet::valid(&units));
            if let Some(unit) = units.get_mut(offset) { *unit = replacement; }
            // SAFETY: mutation changes content, never the allocated buffer length.
            let found = unsafe { sprawling_native_packet_valid(units.as_ptr(), units.len()) };
            prop_assert_eq!(found != 0, packet::valid(&units));
        }
    }

    proptest! {
        #[test]
        fn native_packet_boundary_matches_rust_reference(units in proptest::collection::vec(any::<u16>(), 0..2048)) {
            // SAFETY: the generated vector is live for this call, with its exact length.
            let found = unsafe { sprawling_native_packet_valid(units.as_ptr(), units.len()) };
            prop_assert_eq!(found != 0, packet::valid(&units));
        }
    }

    use std::os::windows::ffi::OsStringExt;

    const ARGV_SEED: u64 = 20_261_006;

    fn argv_launch(args: Vec<OsString>) -> Launch {
        Launch {
            program: PathBuf::from(r"C:\argv probe\argv child.exe"),
            args,
            directory: PathBuf::from(r"C:\argv probe"),
            profile: "sprawling.argv".to_owned(),
            stdout: PathBuf::from("out"),
            stderr: PathBuf::from("err"),
            environment: BTreeMap::new(),
            cpu_rate: NonZeroU16::MIN,
            parent_job: NonZeroUsize::MIN,
        }
    }

    #[test]
    fn native_cmd_script_uses_source_instead_of_crt_arguments() {
        for args in [
            vec!["/C", "exit 3"],
            vec!["/C", "type", "build.log"],
            vec!["/C", "echo written> made.txt && type made.txt"],
            vec!["/C", "echo \"quoted text\" & echo %PATH%"],
            vec!["/d", "/s", "/c", "echo tail\\"],
        ] {
            let mut request = argv_launch(args.iter().map(OsString::from).collect());
            request.program = PathBuf::from(r"C:\Windows\System32\cmd.exe");
            let encoded = packet::encode(&request).unwrap();
            let command = encoded.split(|unit| *unit == 0).nth(1).unwrap();
            let boundary = args
                .iter()
                .position(|arg| arg.eq_ignore_ascii_case("/c"))
                .unwrap();
            let expected = format!(
                "\"{}\" /D /S {} \"{}\"",
                request.program.display(),
                args[..=boundary].join(" "),
                args[boundary + 1..].join(" ")
            );
            assert_eq!(OsString::from_wide(command), OsString::from(expected));
        }
    }

    fn argv_units() -> impl Strategy<Value = Vec<u16>> {
        proptest::collection::vec(
            prop_oneof![
                4 => proptest::sample::select(vec![9_u16, 32, 34, 92]),
                1 => 1_u16..=u16::MAX,
            ],
            0..128,
        )
    }

    fn crt_arguments(command: &[u16]) -> Vec<Vec<u16>> {
        let mut at = 0;
        let mut quoted = false;
        let mut program = Vec::new();
        while at < command.len() {
            match command[at] {
                34 => quoted = !quoted,
                9 | 32 if !quoted => break,
                unit => program.push(unit),
            }
            at += 1;
        }
        let mut args = vec![program];
        loop {
            while at < command.len() && matches!(command[at], 9 | 32) {
                at += 1;
            }
            if at == command.len() {
                return args;
            }
            quoted = false;
            let mut argument = Vec::new();
            while at < command.len() && (quoted || !matches!(command[at], 9 | 32)) {
                let mut slashes = 0;
                while at < command.len() && command[at] == 92 {
                    slashes += 1;
                    at += 1;
                }
                if at < command.len() && command[at] == 34 {
                    argument.extend(std::iter::repeat_n(92, slashes / 2));
                    if slashes % 2 == 1 {
                        argument.push(34);
                    } else if quoted && command.get(at + 1) == Some(&34) {
                        argument.push(34);
                        at += 1;
                    } else {
                        quoted = !quoted;
                    }
                    at += 1;
                } else {
                    argument.extend(std::iter::repeat_n(92, slashes));
                    if at < command.len() && (quoted || !matches!(command[at], 9 | 32)) {
                        argument.push(command[at]);
                        at += 1;
                    }
                }
            }
            args.push(argument);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 256,
            rng_seed: proptest::test_runner::RngSeed::Fixed(ARGV_SEED),
            ..ProptestConfig::default()
        })]

        #[test]
        fn native_encoded_arguments_preserve_units_and_order(
            args in proptest::collection::vec(argv_units(), 0..16),
            program in proptest::collection::vec(
                prop_oneof![
                    4 => proptest::sample::select(vec![9_u16, 32, 92]),
                    1 => (1_u16..=u16::MAX).prop_filter("program has no quotes", |unit| *unit != 34),
                ],
                1..128,
            ).prop_filter("file path has no terminal backslash", |units| units.last() != Some(&92)),
        ) {
            for args in [args, ["/D", "/S", "/C"].into_iter().map(|arg| arg.encode_utf16().collect()).collect()] {
                let mut request = argv_launch(args.iter().map(|units| OsString::from_wide(units)).collect());
                request.program = PathBuf::from(OsString::from_wide(&program));
                let encoded = packet::encode(&request).unwrap();
                let command = encoded.split(|unit| *unit == 0).nth(1).unwrap();
                let mut expected = vec![program.clone()];
                expected.extend(args);
                prop_assert_eq!(crt_arguments(command), expected);
            }
        }

        #[test]
        fn native_cmd_source_preserves_utf16_units(source in argv_units()) {
            let mut request = argv_launch(vec![OsString::from("/C"), OsString::from_wide(&source)]);
            request.program = PathBuf::from(r"C:\Windows\System32\cmd.exe");
            let encoded = packet::encode(&request).unwrap();
            let command = encoded.split(|unit| *unit == 0).nth(1).unwrap();
            let prefix: Vec<u16> = r#""C:\Windows\System32\cmd.exe" /D /S /C ""#.encode_utf16().collect();
            let decoded = command.strip_prefix(prefix.as_slice()).unwrap().strip_suffix(&[34]).unwrap();
            prop_assert_eq!(decoded, source);
        }

        #[test]
        fn native_nul_argument_is_rejected_before_launch(
            mut units in argv_units(), position in 0_usize..129,
        ) {
            let at = position % (units.len() + 1);
            units.insert(at, 0);
            let failure = packet::encode(&argv_launch(vec![OsString::from_wide(&units)])).unwrap_err();
            prop_assert_eq!(failure.action, Action::Encode);
            prop_assert_eq!(failure.code, 87);
            prop_assert!(failure.phase.is_empty());
            prop_assert_eq!(failure.cleanup_code, None);
            prop_assert!(failure.resources.is_none());
        }
    }
}
