; Hooks NSIS de NULL/CONTROL (inclus par le template tauri-bundler 2.10).
; Ils ne servent qu'aux installations et desinstallations manuelles : la mise a jour
; depuis l'appli arrete le moteur et pose le marqueur engine-restart elle-meme.

Var EngineWasRunning

; Arrete le moteur installe : --quit (attend jusqu'a 5 s), taskkill en dernier recours.
!macro NULLCTL_STOP_ENGINE
  ClearErrors
  ExecWait '"$INSTDIR\nullctl-engine.exe" --quit' $0
  ${If} ${Errors}
  ${OrIf} $0 <> 0
    nsExec::Exec '"$SYSDIR\taskkill.exe" /F /IM nullctl-engine.exe'
    Pop $0
    Sleep 500 ; laisse Windows liberer l'exe avant de l'ecraser ou de le supprimer
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  StrCpy $EngineWasRunning 0
  ${If} ${FileExists} "$INSTDIR\nullctl-engine.exe"
    ; Mutex d'instance unique du moteur (engine/src/main.rs).
    System::Call 'kernel32::OpenMutexW(i 0x00100000, i 0, w "Local\nullctl-engine") p .r1'
    ${If} $1 P<> 0
      System::Call 'kernel32::CloseHandle(p r1)'
      StrCpy $EngineWasRunning 1
      !insertmacro NULLCTL_STOP_ENGINE
    ${EndIf}
  ${EndIf}
  ; Ne pas toucher a engine-restart : la mise a jour depuis l'appli en a besoin.
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ${If} $EngineWasRunning = 1
    Exec '"$INSTDIR\nullctl-engine.exe"'
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} ${FileExists} "$INSTDIR\nullctl-engine.exe"
    !insertmacro NULLCTL_STOP_ENGINE
  ${EndIf}
  ; Comme le template : rien de tout ca si un installeur /UPDATE lance la desinstallation.
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "nullctl"
    Delete "$APPDATA\nullctl\engine-restart"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Case « Supprimer les donnees de l'application », decochee par defaut. Le template ne
  ; supprime que les dossiers nommes d'apres l'identifiant ; la config est dans %APPDATA%\nullctl.
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    RMDir /r "$APPDATA\nullctl"
  ${EndIf}
!macroend
