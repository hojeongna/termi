@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0termi-hook.ps1"
exit /b %errorlevel%
