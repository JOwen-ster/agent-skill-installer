!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    ClearErrors
    Delete "$PROFILE\agent-skill-installer.db"
    ; Preserve SQLite recovery files if the database itself could not be removed.
    ${IfNot} ${Errors}
      Delete "$PROFILE\agent-skill-installer.db-wal"
      Delete "$PROFILE\agent-skill-installer.db-shm"
    ${EndIf}
    ${If} ${Errors}
      MessageBox MB_OK|MB_ICONEXCLAMATION "Some history database files could not be removed from $PROFILE. Close any programs using agent-skill-installer.db and remove them manually." /SD IDOK
    ${EndIf}
  ${EndIf}
!macroend
