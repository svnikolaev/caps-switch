@echo off
rem caps-switch: установка двойным кликом.
rem Запускает install.ps1 рядом с этим файлом; параметры передаются дальше.

setlocal
set "SCRIPT=%~dp0install.ps1"
if not exist "%SCRIPT%" (
    echo install.ps1 not found next to install.cmd
    pause
    exit /b 1
)

powershell -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%" %*
pause
