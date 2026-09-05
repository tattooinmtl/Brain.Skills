@echo off
REM install.cmd — Windows native launcher for Brain.Skills
if exist "%~dp0bin\skills.exe" (
  "%~dp0bin\skills.exe" install %*
  exit /b %ERRORLEVEL%
)

where go >nul 2>nul
if not errorlevel 1 (
  echo Compiling native Go launcher...
  go build -o "%~dp0bin\skills.exe" "%~dp0agent-skills-installer\cmd\agent-installer"
  "%~dp0bin\skills.exe" install %*
  exit /b %ERRORLEVEL%
)

echo skills.exe not found in bin\ and Go is not installed.
exit /b 1
