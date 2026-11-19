@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul
title K1tty Tools
cd /d "%~dp0k1tty_editor"
set PYTHONDONTWRITEBYTECODE=1

set "PY="
where python >nul 2>&1 && (
    python --version >nul 2>&1 && set "PY=python"
)
if not defined PY (
    where py >nul 2>&1 && (
        py -3 --version >nul 2>&1 && set "PY=py -3"
    )
)
if not defined PY (
    echo Python 3 not found. Install Python 3 from https://www.python.org/downloads/ and try again.
    pause
    exit /b 1
)

%PY% -B -c "import aiohttp, psutil" >nul 2>&1
if errorlevel 1 (
    echo Installing dependencies ^(aiohttp, psutil^)...
    %PY% -m pip --version >nul 2>&1 || %PY% -m ensurepip --upgrade >nul 2>&1
    %PY% -m pip install --disable-pip-version-check aiohttp psutil
    if errorlevel 1 (
        echo Failed to install dependencies via pip.
        pause
        exit /b 1
    )
    %PY% -B -c "import aiohttp, psutil" >nul 2>&1
    if errorlevel 1 (
        echo Dependencies could not be imported after installation.
        pause
        exit /b 1
    )
)

%PY% -B -X utf8 server.py 8780 %*
if errorlevel 1 (
    echo.
    pause
)
