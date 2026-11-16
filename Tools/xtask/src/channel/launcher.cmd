:; runtime=$(command -v bun); if test -z "$runtime"; then runtime=$(command -v node); fi; if test -z "$runtime"; then echo "sprawling: install Bun or Node.js to run this package"; exit 1; fi ; if test $# -eq 0; then echo "sprawling: package entry file missing; reinstall this package"; exit 1; fi; exec "$runtime" "$@" # Ignore the batch line ending after the shell dispatch.
: <<'WINDOWS'
@echo off
if "%~1"=="" goto noentry
where bun >nul 2>nul
if errorlevel 1 goto node
bun %*
exit /b %errorlevel%
:node
where node >nul 2>nul
if errorlevel 1 goto missing
node %*
exit /b %errorlevel%
:missing
echo sprawling: install Bun or Node.js to run this package 1>&2
exit /b 1
:noentry
echo sprawling: package entry file missing; reinstall this package 1>&2
exit /b 1
WINDOWS
