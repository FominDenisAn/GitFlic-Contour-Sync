@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\Build-Windows.ps1"
if errorlevel 1 (
  echo.
  echo СБОРКА ЗАВЕРШИЛАСЬ С ОШИБКОЙ
  pause
  exit /b 1
)
echo.
echo СБОРКА УСПЕШНА
pause
