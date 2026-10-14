@echo off
rem Tool fixture: a stand-in `cargo` for planner execution tests on Windows.
rem It records its argv and exits per environment; it never compiles anything.
rem   SC_LINT_SHIM_LOG        file that receives one argv line per invocation
rem   SC_LINT_SHIM_NO_XWIN=1  `cargo xwin --version` fails (xwin not installed)
rem   SC_LINT_SHIM_FAIL_VERB  exit 1 when the first argument (or the verb after
rem                           `xwin`) equals this value
>>"%SC_LINT_SHIM_LOG%" echo %*
if "%~1"=="xwin" if "%~2"=="--version" goto xwin_version
set "VERB=%~1"
if "%~1"=="xwin" set "VERB=%~2"
if defined SC_LINT_SHIM_FAIL_VERB if "%VERB%"=="%SC_LINT_SHIM_FAIL_VERB%" (
  echo shim: %VERB% failed as requested 1>&2
  exit /b 1
)
exit /b 0
:xwin_version
if "%SC_LINT_SHIM_NO_XWIN%"=="1" exit /b 1
exit /b 0
