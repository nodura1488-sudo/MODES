; Lavunio installer hooks.
; Migrate an existing TMStore installation in place when the user did not pick
; a custom directory, so the old auto-update restart path keeps working.

!macro NSIS_HOOK_PREINSTALL
  ; Only migrate when the installer is using the new default location.
  ; A custom directory picked by the user must be respected.
  StrCmp $INSTDIR "$LOCALAPPDATA\Lavunio" 0 LavunioInstallDirReady
  ReadRegStr $0 HKCU "Software\TM\TMStore" ""
  StrCmp $0 "$LOCALAPPDATA\TMStore" 0 LavunioInstallDirReady
  StrCpy $INSTDIR $0
  SetOutPath $INSTDIR
  LavunioInstallDirReady:
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Older updaters restart tmstore.exe or tm-launcher.exe. Keep compatibility
  ; copies so the first update after the rename does not leave the app closed.
  IfFileExists "$INSTDIR\lavunio.exe" 0 LavunioCompatDone
    CopyFiles /SILENT "$INSTDIR\lavunio.exe" "$INSTDIR\tmstore.exe"
    CopyFiles /SILENT "$INSTDIR\lavunio.exe" "$INSTDIR\tm-launcher.exe"
  LavunioCompatDone:

  ; Drop the old TMStore uninstall entry if it points to this directory.
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TMStore" "InstallLocation"
  StrCmp $0 $INSTDIR 0 LavunioUninstallKeyReady
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TMStore"
  LavunioUninstallKeyReady:

  ; Remove old shortcuts left by TMStore.
  Delete "$SMPROGRAMS\TMStore.lnk"
  Delete "$DESKTOP\TMStore.lnk"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\tmstore.exe"
  Delete "$INSTDIR\tm-launcher.exe"
!macroend
