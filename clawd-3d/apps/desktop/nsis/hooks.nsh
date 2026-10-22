; Clawd's installer hooks (Tauri's NSIS installer). Clawd for Windows was an Electron app up to v1.22, installed in
; %LOCALAPPDATA%\Programs\clawd-desktop. Its updater installs this Rust version like any update, so the first Rust
; install takes the Electron one out: one Clawd in the Start menu, one at login. Its settings are read by the new app.
!include "FileFunc.nsh"

!define CLAWD_ELECTRON_DIR "$LOCALAPPDATA\Programs\clawd-desktop"

!macro NSIS_HOOK_PREINSTALL
  IfFileExists "${CLAWD_ELECTRON_DIR}\Uninstall Clawd.exe" 0 clawd_no_electron
    ; _?= runs the uninstaller in place and waits for it.
    ExecWait '"${CLAWD_ELECTRON_DIR}\Uninstall Clawd.exe" /S _?=${CLAWD_ELECTRON_DIR}'
    RMDir /r "${CLAWD_ELECTRON_DIR}"
  clawd_no_electron:
  ; The Electron app's "Start with Windows" entry (the new app adds its own if the setting was on).
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "com.dekelelely.clawd"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "electron.app.Clawd"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Silent updates that ask for it start Clawd again afterwards: the Electron app's updater and this app's
  ; "Restart to update" both pass --force-run.
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "--force-run" $R1
  IfErrors clawd_no_run 0
    Exec '"$INSTDIR\Clawd.exe"'
  clawd_no_run:
!macroend
