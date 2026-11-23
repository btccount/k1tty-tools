@echo off
setlocal EnableExtensions DisableDelayedExpansion
chcp 65001 >nul 2>&1
cd /d "%~dp0" || exit /b 1

set "REPO=quesada-lab/k1tty-tools"

:MODE_MENU
cls
echo.
echo   ===========================================
echo                K1TTY TOOLS

echo               Desktop Build Menu

echo   ===========================================
echo.
echo   [1] DEV build           - debug binaries/installers

echo   [2] PRODUCTION build    - optimized release/installers

echo   [3] LIVE DEV            - run Windows app with Tauri

echo   [0] Exit

echo.
choice /C 1230 /N /M "Choose build mode [1/2/3/0]: "
if errorlevel 4 goto FINISH
if errorlevel 3 goto LIVE_DEV
if errorlevel 2 goto SET_PROD
set "MODE=debug"
goto TARGET_MENU

:SET_PROD
set "MODE=production"
goto TARGET_MENU

:TARGET_MENU
cls
echo.
echo   MODE: %MODE%
echo.
echo   [1] Windows x64       - local build on this PC

echo   [2] Linux x64         - GitHub Actions (Ubuntu)

echo   [3] macOS Universal   - GitHub Actions (Intel + Apple Silicon)

echo   [4] All three OS      - GitHub Actions

echo   [0] Back

echo.
choice /C 12340 /N /M "Choose target OS [1/2/3/4/0]: "
if errorlevel 5 goto MODE_MENU
if errorlevel 4 goto ALL_OS
if errorlevel 3 goto MAC_OS
if errorlevel 2 goto LINUX_OS
set "TARGET=windows"
goto WINDOWS_BUILD

:LINUX_OS
set "TARGET=linux"
goto REMOTE_BUILD

:MAC_OS
set "TARGET=macos"
goto REMOTE_BUILD

:ALL_OS
set "TARGET=all"
goto REMOTE_BUILD

:LIVE_DEV
call :CHECK_LOCAL_TOOLS
if errorlevel 1 goto PAUSE_MENU
call :INSTALL_NPM
if errorlevel 1 goto PAUSE_MENU
echo.
echo [RUN] npm run dev
call npm run dev
if errorlevel 1 echo [ERROR] Tauri development run failed.
goto PAUSE_MENU

:WINDOWS_BUILD
call :CHECK_LOCAL_TOOLS
if errorlevel 1 goto PAUSE_MENU
call :INSTALL_NPM
if errorlevel 1 goto PAUSE_MENU
echo.
if "%MODE%"=="debug" goto WINDOWS_DEBUG

echo [BUILD] Windows Production - NSIS and MSI
call npm run build -- --bundles nsis,msi
if errorlevel 1 goto BUILD_FAILED
echo.
echo [OK] Production build output:
echo %CD%\target\release\bundle
goto PAUSE_MENU

:WINDOWS_DEBUG
echo [BUILD] Windows Dev/Debug - NSIS and MSI
call npm run build -- --debug --bundles nsis,msi
if errorlevel 1 goto BUILD_FAILED
echo.
echo [OK] Debug build output:
echo %CD%\target\debug\bundle
goto PAUSE_MENU

:BUILD_FAILED
echo.
echo [ERROR] Build failed. Read the compiler output above.
goto PAUSE_MENU

:REMOTE_BUILD
cls
echo.
echo [INFO] Target: %TARGET%  Mode: %MODE%
echo [INFO] Building Linux/macOS from Windows requires native GitHub runners.
echo [INFO] You must first commit and push THIS version of the project,
echo        including .github\workflows\desktop.yml, to your GitHub repo.
echo.
where gh >nul 2>&1
if errorlevel 1 goto GH_MISSING

gh auth status >nul 2>&1
if errorlevel 1 goto GH_LOGIN

set "BRANCH="
where git >nul 2>&1
if errorlevel 1 goto ASK_BRANCH
for /f "delims=" %%G in ('git branch --show-current 2^>nul') do set "BRANCH=%%G"
:ASK_BRANCH
if not defined BRANCH set /p "BRANCH=Git branch containing these sources [main]: "
if not defined BRANCH set "BRANCH=main"
echo.
echo [REMOTE] Repo:   %REPO%
echo [REMOTE] Branch: %BRANCH%
echo [REMOTE] Target: %TARGET%
echo [REMOTE] Mode:   %MODE%
echo.
choice /C YN /N /M "Trigger GitHub Actions now? [Y/N]: "
if errorlevel 2 goto PAUSE_MENU

gh workflow run desktop.yml --repo "%REPO%" --ref "%BRANCH%" -f "target_os=%TARGET%" -f "build_mode=%MODE%"
if errorlevel 1 goto REMOTE_FAILED
echo.
echo [OK] GitHub Actions build dispatched.
echo [INFO] Open: https://github.com/%REPO%/actions/workflows/desktop.yml
echo [INFO] Download the generated files from the workflow Artifacts section.
goto PAUSE_MENU

:REMOTE_FAILED
echo.
echo [ERROR] Could not dispatch the workflow.
echo [HINT] The updated workflow must be pushed to the default branch first.
echo [HINT] Your account must have Actions permission for this repository.
goto PAUSE_MENU

:GH_MISSING
echo [ERROR] GitHub CLI (gh) not found.
echo Install it: winget install --id GitHub.cli -e
echo Then open a new CMD, run gh auth login, and retry.
goto PAUSE_MENU

:GH_LOGIN
echo [ERROR] GitHub CLI is not authenticated.
echo Run: gh auth login
goto PAUSE_MENU

:CHECK_LOCAL_TOOLS
where node >nul 2>&1
if errorlevel 1 goto MISSING_NODE
where npm >nul 2>&1
if errorlevel 1 goto MISSING_NODE
where cargo >nul 2>&1
if errorlevel 1 goto MISSING_CARGO
exit /b 0
:MISSING_NODE
echo [ERROR] Node.js/npm not found in PATH.
exit /b 1
:MISSING_CARGO
echo [ERROR] Cargo not found. Install Rust from https://rustup.rs/
echo Then open a new CMD (or run: set "PATH=%%USERPROFILE%%\.cargo\bin;%%PATH%%").
exit /b 1

:INSTALL_NPM
if exist "node_modules\@tauri-apps\cli" exit /b 0
echo [SETUP] Installing dependencies: npm ci
call npm ci
exit /b %errorlevel%

:PAUSE_MENU
echo.
pause
goto MODE_MENU

:FINISH
endlocal
exit /b 0
