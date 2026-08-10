@echo off
echo Building Release Executable via Cargo...
cargo build --release
if %errorlevel% neq 0 exit /b %errorlevel%

echo.
echo Building Installable Setup via Inno Setup...
mkdir downloads 2>nul
"C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer.iss
if %errorlevel% neq 0 exit /b %errorlevel%

echo.
echo =======================================================
echo Success! Installable App is ready in the 'downloads' folder.
echo =======================================================
pause
