; Migrate a previous TMLauncher installation into TMStore in place.
; This keeps the auto-update restart path valid and avoids a duplicate install.

!macro NSIS_HOOK_PREINSTALL
  ; Only migrate when the installer is using the new default location.
  ; A custom directory picked by the user must be respected.
  StrCmp $INSTDIR "$LOCALAPPDATA\TMStore" 0 TMStoreInstallDirReady
  ReadRegStr $0 HKCU "Software\TM\TMStore" ""
  StrCmp $0 "" 0 TMStoreInstallDirReady
  ReadRegStr $0 HKCU "Software\TM\TMLauncher" ""
  StrCmp $0 "" TMStoreInstallDirReady
  StrCpy $INSTDIR $0
  SetOutPath $INSTDIR
  TMStoreInstallDirReady:
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $0 HKCU "Software\TM\TMStore" ""
  StrCmp $0 "" TMStoreNoLegacyKeys
  ReadRegStr $1 HKCU "Software\TM\TMLauncher" ""
  StrCmp $1 "" TMStoreNoLegacyKeys
  StrCmp $0 $1 0 TMStoreNoLegacyKeys
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TMLauncher"
  DeleteRegKey HKCU "Software\TM\TMLauncher"
  TMStoreNoLegacyKeys:
!macroend
