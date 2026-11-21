import React from "react";
import { AlertTriangle, Download, HelpCircle, RefreshCw, RotateCcw, RotateCw, Settings, X } from "lucide-react";
import { deckforgeApi } from "./bridge/tauriBridge";
import { savedStatus } from "./lib/saveQueue";
import { QuitDialog } from "./components/QuitDialog";
import { IconStudio } from "./components/IconStudio";
import { ThemesScreen } from "./screens/ThemesScreen";
import { GameModeStudio } from "./components/GameModeStudio";
import { MacroStudio } from "./components/MacroStudio";
import { PageTemplates } from "./components/PageTemplates";
import { WorkspaceStatusBar } from "./components/WorkspaceStatusBar";
import { OnboardingModal } from "./components/OnboardingModal";
import { WorkspaceToolbar } from "./components/WorkspaceToolbar";
import { DeckStage } from "./components/DeckStage";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import { ActionDrawer, CatalogProvider } from "./components/ActionDrawer";
import { KeyInspector } from "./components/KeyInspector";
import { DevicesScreen } from "./screens/DevicesScreen";
import { PluginsScreen } from "./screens/PluginsScreen";
import { SettingsScreen } from "./screens/SettingsScreen";
import { NavScreenId, ScreenTarget } from "./types/deckforge";
import { NAV_ITEMS } from "./app/navItems";
import { useDeckEditor } from "./app/useDeckEditor";

export const App: React.FC = () => {
  const {
    activeScreen,
    setActiveScreen,
    devices,
    setDevices,
    profiles,
    autoSwitchRules,
    macros,
    diagnostics,
    frameStats,
    legacySummary,
    telemetry,
    setSelectedDeviceId,
    setSelectedProfileId,
    startupProfileId,
    setStartupProfileId,
    selectedPageIdx,
    setSelectedPageIdx,
    setSelectedKeyIdx,
    surface,
    setSurface,
    surfaceIndex,
    setSurfaceIndex,
    mediaLibrary,
    setMediaLibrary,
    fileRef,
    inspectorTab,
    setInspectorTab,
    actionCategoryFilter,
    setActionCategoryFilter,
    canvasZoom,
    setCanvasZoom,
    clipboardKey,
    setClipboardKey,
    templatesOpen,
    setTemplatesOpen,
    savedLocally,
    recoveryChecked,
    setRecoveryChecked,
    recoveryDraft,
    recoveryError,
    setRecoveryError,
    recoveryBusy,
    draftStatus,
    draftWriter,
    quitOpen,
    setQuitOpen,
    quitting,
    quitError,
    saveStatuses,
    bootstrapError,
    setBootstrapAttempt,
    saveQueues,
    appliedRefresh,
    undoStack,
    redoStack,
    toastMessage,
    setToastMessage,
    selectedMacroId,
    setSelectedMacroId,
    selectedMacroStepIdx,
    setSelectedMacroStepIdx,
    macroRunReport,
    setMacroRunReport,
    showOnboarding,
    setShowOnboarding,
    livePressedKeyIdx,
    liveEncoderEvent,
    liveTouchZoneIdx,
    face,
    obsSettings,
    triggerToast,
    resolveRecovery,
    finishQuit,
    handleRescanDevices,
    handleSelectDevice,
    handleImportLegacyProfiles,
    hotkeyTarget,
    setHotkeyTarget,
    setCustomActions,
    plugins,
    setPlugins,
    allActionTemplates,
    connectedDevices,
    activeDevice,
    activeProfile,
    activePage,
    activeKey,
    paramOptionsFor,
    saveRules,
    saveObs,
    persistMacros,
    assignMacroToKey,
    activeZone,
    activeEncoder,
    handleUndo,
    handleRedo,
    updateSelectedKey,
    clipFor,
    assignMedia,
    currentMedia,
    patchCurrentMedia,
    ingestFile,
    applyKeyFrames,
    useSampleAnimation,
    updateSelectedZone,
    updateSelectedEncoder,
    handleSaveLocally,
    handleApplyToDevice,
    handleTestSelectedAction,
    handleClearSelectedKey,
    handleBrightnessChange,
    handleKnobLedChange,
    handleResetKnobLeds,
    handleApplyKnobLedPalette,
    handleKnobLedEffectChange,
    handleApplyKnobLedsToAllPages,
    ledBrightness,
    ledFollowSleep,
    handleLedBrightnessChange,
    handleLedSleepModeChange,
    handleRestartDeck,
    handleAddProfile,
    handleBackup,
    handleRestoreBackup,
    handleAddPage,
    handleAddPageFromTemplate,
    handleReplacePageFromTemplate,
    handleBatchApplyStyleToPage,
    drawerCallbacks,
    stageCallbacks,
    sidebarCallbacks,
  } = useDeckEditor();
  const quitDialog = <QuitDialog open={quitOpen} busy={quitting} error={quitError}
    dirty={!savedLocally || !!recoveryDraft || !!recoveryError || Object.values(saveStatuses).some(s => s.pending)}
    onCancel={() => setQuitOpen(false)} onQuit={finishQuit} />;

  if (bootstrapError) {
    return (
      <div className="h-screen bg-[var(--bg-app)] text-[var(--text-primary)] flex items-center justify-center p-8" role="alert">
        {quitDialog}
        <div className="max-w-md w-full rounded-2xl border border-[var(--border-strong)] bg-[var(--bg-panel)] p-6 shadow-2xl space-y-4 text-center animate-pop-in">
          <div className="w-11 h-11 mx-auto rounded-xl bg-[var(--status-warning)]/15 text-[var(--status-warning)] flex items-center justify-center">
            <AlertTriangle size={22} />
          </div>
          <h1 className="text-lg font-bold">Could not load DeckForge</h1>
          <p className="text-xs leading-relaxed text-[var(--status-warning)] break-words">{bootstrapError}</p>
          <button
            type="button"
            className="h-9 px-5 rounded-lg bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white cursor-pointer"
            onClick={() => setBootstrapAttempt(n => n + 1)}
          >
            Retry loading
          </button>
        </div>
      </div>
    );
  }

  if (recoveryChecked && (recoveryDraft || recoveryError)) {
    return (
      <div className="h-screen bg-[var(--bg-app)] text-[var(--text-primary)] flex items-center justify-center p-8">
        {quitDialog}
        <section aria-labelledby="recovery-title" className="max-w-lg w-full rounded-2xl border border-[var(--border-strong)] bg-[var(--bg-panel)] p-6 shadow-2xl space-y-4 animate-pop-in">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-[var(--accent-soft)] text-[var(--accent-text)] flex items-center justify-center shrink-0">
              <AlertTriangle size={20} />
            </div>
            <h1 id="recovery-title" className="text-lg font-bold">{recoveryDraft ? "Recover unsaved edits?" : "Could not read recovery draft"}</h1>
          </div>
          {recoveryDraft && <p className="text-xs leading-relaxed text-[var(--text-secondary)]">{recoveryDraft.draft.profiles.length} profiles captured {new Date(recoveryDraft.draft.updatedAtMs).toLocaleString()}. Restore opens them as unsaved editor drafts. Saved profiles and the deck stay unchanged until you Save or Apply.</p>}
          {recoveryDraft?.savedProfilesChanged && <p className="text-xs leading-relaxed text-[var(--status-warning)] bg-[var(--status-warning)]/10 border border-[var(--status-warning)]/30 rounded-lg px-3 py-2">Saved profiles have changed since this draft began. Restoring replaces the editor contents with the recovered version; review it before saving. Undo returns to the loaded saved profiles.</p>}
          {recoveryError && <p role="alert" className="text-xs text-[var(--status-error)] break-words">{recoveryError}</p>}
          {!recoveryDraft && <p className="text-xs leading-relaxed text-[var(--text-secondary)]">The file is preserved at <code className="text-[var(--text-primary)]">%APPDATA%\DeckForge\profile-draft.json</code>. Retry reading it, or discard this recovery copy to open saved profiles.</p>}
          <div className="flex flex-wrap gap-2.5 pt-1 text-xs">
            {recoveryDraft ? (
              <button type="button" autoFocus disabled={recoveryBusy} className="h-9 px-4 rounded-lg bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] font-semibold text-white cursor-pointer disabled:opacity-40" onClick={() => resolveRecovery(true)}>Restore draft</button>
            ) : (
              <button type="button" disabled={recoveryBusy} className="h-9 px-4 rounded-lg border border-[var(--border-strong)] hover:border-[var(--accent-primary)] font-medium cursor-pointer disabled:opacity-40" onClick={() => { setRecoveryChecked(false); setRecoveryError(null); setBootstrapAttempt(n => n + 1); }}>Retry recovery</button>
            )}
            <button type="button" disabled={recoveryBusy} className="h-9 px-4 rounded-lg border border-[var(--border-strong)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-elevated)] font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer disabled:opacity-40" onClick={() => resolveRecovery(false)}>Discard draft</button>
          </div>
        </section>
      </div>
    );
  }

  if (!recoveryChecked || !activeDevice || !activeProfile || !activePage || !activeKey) {
    return (
      <div className="h-screen w-screen bg-[var(--bg-app)] text-[var(--text-primary)] flex items-center justify-center p-6">
        {quitDialog}
        <div className="flex flex-col items-center gap-4 text-center animate-fade-in">
          <div aria-hidden className="w-10 h-10 rounded-xl bg-[var(--accent-primary)] p-2 grid grid-cols-2 gap-1 shadow-[0_4px_20px_rgba(124,131,247,0.35)]">
            <span className="bg-[var(--bg-app)] rounded-[2px]" />
            <span className="bg-[var(--bg-app)] rounded-[2px]" />
            <span className="bg-[var(--bg-app)] rounded-[2px]" />
            <span className="bg-[var(--bg-app)] rounded-[2px] opacity-50" />
          </div>
          <div className="flex items-center gap-2.5 text-xs text-[var(--text-secondary)]">
            <RefreshCw className="animate-spin text-[var(--accent-primary)]" size={15} />
            <span>Initializing DeckForge Hardware &amp; Action Service…</span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen w-screen bg-[var(--bg-app)] text-[var(--text-primary)] flex flex-col overflow-hidden select-none">
      {quitDialog}
      {(!savedLocally || draftStatus.error) && (
        <div role={draftStatus.error ? "alert" : "status"} className="px-4 py-1.5 text-[11.5px] bg-[var(--bg-panel)] border-b border-[var(--border-subtle)] text-[var(--text-secondary)] flex items-center gap-2.5 shrink-0">
          <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${draftStatus.error ? "bg-[var(--status-warning)]" : draftStatus.pending ? "bg-[var(--accent-primary)] animate-pulse" : "bg-[var(--status-success)]"}`} />
          <span className="truncate">{draftStatus.error ? `Recovery copy could not update: ${draftStatus.error}. Retry recovery or save your profiles.` : draftStatus.pending ? "Updating recovery copy…" : "Recovery copy saved · Profile edits are still unsaved"}</span>
          {draftStatus.error && <button type="button" className="ml-auto shrink-0 px-2.5 py-0.5 rounded-md border border-[var(--status-warning)]/40 bg-[var(--status-warning)]/10 text-[var(--status-warning)] hover:bg-[var(--status-warning)]/20 hover:text-[var(--text-primary)] cursor-pointer font-medium transition-colors" onClick={draftWriter.retry}>Retry recovery copy</button>}
        </div>
      )}
      {Object.entries(saveQueues).map(([name, queue]) => {
        const status = saveStatuses[name] ?? savedStatus;
        if (!status.pending) return null;
        return (
          <div key={name} role={status.error ? "alert" : "status"} className="px-4 py-1.5 text-[11.5px] bg-[var(--accent-soft)] border-b border-[var(--accent-primary)]/30 text-[var(--status-warning)] flex items-center gap-2.5 shrink-0">
            <span className="w-1.5 h-1.5 rounded-full shrink-0 bg-current animate-pulse" />
            <span className="truncate">{status.error ? `${name} not saved: ${status.error}. Your edits remain in this session.` : `Saving ${name.toLowerCase()}…`}</span>
            {status.error && <button type="button" className="ml-auto shrink-0 px-2.5 py-0.5 rounded-md border border-[var(--status-warning)]/40 bg-[var(--status-warning)]/10 text-[var(--status-warning)] hover:bg-[var(--status-warning)]/20 hover:text-white font-semibold cursor-pointer transition-colors" onClick={queue.retry}>Retry save</button>}
          </div>
        );
      })}
      <input
        ref={fileRef}
        type="file"
        accept="image/png,image/jpeg,image/gif,image/webp,image/bmp,.gif"
        className="hidden"
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = "";
          if (!file) return;
          if (surface === "encoder") {
            triggerToast("Choose a key or touch screen first", "warning");
            return;
          }
          const target: ScreenTarget = surface === "zone" || surface === "strip" ? surface : "key";
          const index = target === "key" ? activeKey.index : surfaceIndex;
          void ingestFile(file, target, index);
        }}
      />
      {/* Transient Bottom-Right Toast */}
      {toastMessage && (
        <div
          role="status"
          aria-live="polite"
          className={`fixed bottom-14 right-5 z-50 flex items-center gap-2.5 pl-3.5 pr-2.5 py-2.5 rounded-xl bg-[var(--bg-elevated)]/95 backdrop-blur-md border shadow-2xl text-[12.5px] font-medium text-[var(--text-primary)] max-w-md animate-slide-up ${
            toastMessage.kind === "warning"
              ? "border-[var(--status-warning)]/45"
              : toastMessage.kind === "info"
              ? "border-[var(--accent-primary)]/40"
              : "border-[var(--status-success)]/40"
          }`}
        >
          <span
            className={`w-2 h-2 rounded-full shrink-0 ${
              toastMessage.kind === "warning" ? "bg-[var(--status-warning)]" : toastMessage.kind === "info" ? "bg-[var(--accent-primary)]" : "bg-[var(--status-success)]"
            }`}
          />
          <span className="leading-snug">{toastMessage.text}</span>
          {toastMessage.action && (
            <button
              type="button"
              onClick={() => {
                toastMessage.action?.run();
                setToastMessage(null);
              }}
              className="ml-1 px-2.5 h-6 rounded-[var(--radius-sm)] bg-[var(--accent-soft)] text-[var(--accent-text)] hover:bg-[var(--accent-primary)] hover:text-white font-semibold cursor-pointer shrink-0"
            >
              {toastMessage.action.label}
            </button>
          )}
          <button
            type="button"
            aria-label="Dismiss notification"
            onClick={() => setToastMessage(null)}
            className="p-1 rounded-md text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)] cursor-pointer shrink-0"
          >
            <X size={13} />
          </button>
        </div>
      )}
      {templatesOpen && (
        <PageTemplates
          currentPage={activePage}
          profileName={activeProfile.name}
          onAdd={handleAddPageFromTemplate}
          onReplace={handleReplacePageFromTemplate}
          onClose={() => setTemplatesOpen(false)}
          notify={triggerToast}
        />
      )}

      <div className="flex-1 min-h-0 flex">
      {/* Icon rail: one place to switch screens, labels on hover. */}
      <nav aria-label="Screens" className="w-14 shrink-0 bg-[var(--bg-sidebar)] border-r border-[var(--border-subtle)] flex flex-col items-center gap-1.5 py-3">
        <div aria-hidden className="w-7 h-7 mb-2.5 rounded-[var(--radius-sm)] bg-[var(--accent-primary)] p-[5px] grid grid-cols-2 gap-[3px] shadow-[0_2px_10px_rgba(124,131,247,0.3)]">
          <span className="bg-[var(--bg-app)] rounded-[1.5px]" />
          <span className="bg-[var(--bg-app)] rounded-[1.5px]" />
          <span className="bg-[var(--bg-app)] rounded-[1.5px]" />
          <span className="bg-[var(--bg-app)] rounded-[1.5px] opacity-50" />
        </div>
        {(
          [...(NAV_ITEMS), { id: "settings", label: "Settings", icon: Settings }] as Array<{ id: NavScreenId; label: string; icon: React.ComponentType<{ size?: number; strokeWidth?: number }> }>
        ).map((tab) => {
          const Icon = tab.icon;
          const isActive = activeScreen === tab.id;
          return (
            <React.Fragment key={tab.id}>
              {tab.id === "settings" && <div aria-hidden className="mt-auto mb-1 w-6 h-px bg-[var(--border-subtle)]" />}
              <button
                type="button"
                title={tab.label}
                aria-label={tab.label}
                aria-current={isActive ? "page" : undefined}
                onClick={() => {
                  setActiveScreen(tab.id);
                  if (tab.id === "icons") {
                    setSurface("key");
                    setSurfaceIndex(activeKey.index);
                  }
                }}
                className={`group relative w-10 h-10 rounded-[var(--radius-md)] flex items-center justify-center transition-all duration-150 cursor-pointer ${
                  isActive
                    ? "bg-[var(--bg-elevated)] text-[var(--text-primary)] border border-[var(--border-strong)]/70 shadow-xs"
                    : "border border-transparent text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"
                }`}
              >
                {isActive && <span aria-hidden className="absolute -left-[9px] top-2.5 bottom-2.5 w-[3px] rounded-r-full bg-[var(--accent-primary)] shadow-[0_0_8px_rgba(124,131,247,0.65)]" />}
                <Icon size={18} strokeWidth={isActive ? 2.1 : 1.75} />
                <span
                  aria-hidden
                  className="pointer-events-none absolute left-full ml-3 z-50 px-2.5 py-1 rounded-md bg-[var(--bg-elevated)] border border-[var(--border-strong)] shadow-xl text-[11.5px] font-medium text-[var(--text-primary)] whitespace-nowrap opacity-0 -translate-x-1 group-hover:opacity-100 group-hover:translate-x-0 group-focus-visible:opacity-100 group-focus-visible:translate-x-0 transition-all duration-150"
                >
                  {tab.label}
                </span>
              </button>
            </React.Fragment>
          );
        })}
      </nav>
      <div className="flex-1 min-w-0 min-h-0 flex flex-col">
      <header className="h-12 bg-[var(--bg-app)] border-b border-[var(--border-subtle)] px-4 flex items-center justify-between gap-3 shrink-0">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-[12px] font-medium text-[var(--text-muted)] hidden sm:inline">DeckForge</span>
          <span className="text-[var(--border-strong)] hidden sm:inline" aria-hidden>/</span>
          <span className="text-[13.5px] font-semibold tracking-tight text-[var(--text-primary)] truncate">
            {[...NAV_ITEMS, { id: "settings", label: "Settings" }].find((n) => n.id === activeScreen)?.label ?? "DeckForge"}
          </span>
        </div>
        {/* Right Controls */}
        <div className="flex items-center gap-1.5">
          {telemetry?.officialStreamdockRunning && (
            <span
              title="AJAZZ detected. Close it, then choose Use DeckForge in the status bar."
              className="hidden sm:inline-flex items-center gap-1.5 px-2.5 h-7 rounded-full bg-[var(--status-warning)]/12 border border-[var(--status-warning)]/35 text-[11px] font-medium text-[var(--status-warning)] max-w-[260px] truncate"
            >
              <AlertTriangle size={12} className="shrink-0" />
              <span className="truncate">AJAZZ running — close &amp; choose Use DeckForge</span>
            </span>
          )}
          <button
            type="button"
            onClick={handleImportLegacyProfiles}
            title="Parse & Import Legacy .sdProfile Configurations from %APPDATA%\HotSpot\StreamDock\profiles"
            className="h-8 px-2.5 rounded-[var(--radius-sm)] border border-[var(--border-subtle)] bg-[var(--bg-panel)]/70 hover:border-[var(--border-strong)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] text-[12px] font-medium flex items-center gap-1.5 cursor-pointer transition-colors"
          >
            <Download size={13} />
            <span>Import from Stream Dock</span>
          </button>
          <span className="w-px h-4 bg-[var(--border-subtle)] mx-0.5" aria-hidden />
          <button
            type="button"
            onClick={handleUndo}
            disabled={undoStack.length === 0}
            title="Undo (Ctrl+Z)"
            aria-label="Undo"
            className="w-8 h-8 rounded-[var(--radius-sm)] border border-transparent hover:border-[var(--border-subtle)] flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)] disabled:opacity-35 disabled:hover:border-transparent disabled:hover:bg-transparent disabled:cursor-default cursor-pointer transition-colors"
          >
            <RotateCcw size={14} />
          </button>
          <button
            type="button"
            onClick={handleRedo}
            disabled={redoStack.length === 0}
            title="Redo (Ctrl+Y)"
            aria-label="Redo"
            className="w-8 h-8 rounded-[var(--radius-sm)] border border-transparent hover:border-[var(--border-subtle)] flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)] disabled:opacity-35 disabled:hover:border-transparent disabled:hover:bg-transparent disabled:cursor-default cursor-pointer transition-colors"
          >
            <RotateCw size={14} />
          </button>
          <button
            type="button"
            onClick={() => setShowOnboarding(true)}
            title="Quick-start guide"
            aria-label="Quick-start guide"
            className="w-8 h-8 rounded-[var(--radius-sm)] border border-transparent hover:border-[var(--border-subtle)] flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)] cursor-pointer transition-colors"
          >
            <HelpCircle size={15} />
          </button>
        </div>
      </header>

      {/* 2. SCREEN 01: WORKSPACE / KEY EDITOR (Matches Reference Screenshot + Spec Section 4) */}
      {activeScreen === "workspace" && (
        <CatalogProvider>
        <div key="workspace" className="flex-1 min-h-0 grid grid-cols-[232px_minmax(0,1fr)_292px] 2xl:grid-cols-[256px_minmax(0,1fr)_324px] overflow-hidden animate-screen-in">
          {/* LEFT CONTEXTUAL SIDEBAR: Devices + Profiles */}
          <WorkspaceSidebar activeProfile={activeProfile} autoSwitchRules={autoSwitchRules} connectedDevices={connectedDevices} handleAddProfile={sidebarCallbacks.handleAddProfile} onExportProfile={sidebarCallbacks.onExportProfile} onImportProfile={sidebarCallbacks.onImportProfile} profiles={profiles} macros={macros} selectedPageIdx={selectedPageIdx} saveRules={sidebarCallbacks.saveRules} setDevices={setDevices} setSelectedKeyIdx={setSelectedKeyIdx} setSelectedPageIdx={setSelectedPageIdx} setSelectedProfileId={setSelectedProfileId} setSurface={setSurface} setSurfaceIndex={setSurfaceIndex} triggerToast={sidebarCallbacks.triggerToast} />

          {/* CENTER WORKSPACE COLUMN: Toolbar + Hardware Bezel + Bottom Action Library Drawer */}
          <main className="min-h-0 flex flex-col justify-between bg-[var(--bg-app)] overflow-hidden">
            {/* Top Workspace Toolbar */}
            <WorkspaceToolbar activeDevice={activeDevice} activeProfile={activeProfile} canvasZoom={canvasZoom} connectedDevices={connectedDevices} handleApplyToDevice={handleApplyToDevice} handleSaveLocally={handleSaveLocally} setCanvasZoom={setCanvasZoom} setSelectedDeviceId={setSelectedDeviceId} setSelectedKeyIdx={setSelectedKeyIdx} setSurface={setSurface} setSurfaceIndex={setSurfaceIndex} />

            {/* Central Hardware Bezel Stage */}
            <DeckStage activeDevice={activeDevice} activeKey={activeKey} activePage={activePage} activeProfile={activeProfile} canvasZoom={canvasZoom} clipFor={stageCallbacks.clipFor} face={face} handleAddPage={stageCallbacks.handleAddPage} ingestFile={stageCallbacks.ingestFile} livePressedKeyIdx={livePressedKeyIdx} liveEncoderEvent={liveEncoderEvent} liveTouchZoneIdx={liveTouchZoneIdx} selectedPageIdx={selectedPageIdx} setSelectedKeyIdx={setSelectedKeyIdx} setSelectedPageIdx={setSelectedPageIdx} setSurface={setSurface} setSurfaceIndex={setSurfaceIndex} setTemplatesOpen={setTemplatesOpen} surface={surface} surfaceIndex={surfaceIndex} onDropAction={stageCallbacks.onDropAction} onMoveKey={stageCallbacks.onMoveKey} onMoveKeyToPage={stageCallbacks.onMoveKeyToPage} autoSwitchRules={autoSwitchRules} saveRules={stageCallbacks.saveRules} />

            {/* BOTTOM ACTION LIBRARY DRAWER (Matches Reference Screenshot Bottom Panel) */}
            <ActionDrawer actionCategoryFilter={actionCategoryFilter} activeEncoder={activeEncoder} activeKey={activeKey} activeZone={activeZone} actions={allActionTemplates} setActionCategoryFilter={setActionCategoryFilter} surface={surface} {...drawerCallbacks} />
          </main>

          {/* RIGHT INSPECTOR PANE (320px — Button Properties matching Reference Screenshot) */}
          <KeyInspector mediaLibrary={mediaLibrary} onMediaStored={clip => setMediaLibrary(prev => ({ ...prev, [clip.id]: clip }))} activeDevice={activeDevice} activeEncoder={activeEncoder} activeKey={activeKey} activeZone={activeZone} allActionTemplates={allActionTemplates} assignMedia={assignMedia} clipFor={clipFor} clipboardKey={clipboardKey} currentMedia={currentMedia} face={face} fileRef={fileRef} handleClearSelectedKey={handleClearSelectedKey} handleTestSelectedAction={handleTestSelectedAction} hotkeyTarget={hotkeyTarget} ingestFile={ingestFile} inspectorTab={inspectorTab} macros={macros} obsSettings={obsSettings} paramOptionsFor={paramOptionsFor} patchCurrentMedia={patchCurrentMedia} saveObs={saveObs} setActionCategoryFilter={setActionCategoryFilter} setClipboardKey={setClipboardKey} setCustomActions={setCustomActions} setHotkeyTarget={setHotkeyTarget} setInspectorTab={setInspectorTab} surface={surface} surfaceIndex={surfaceIndex} triggerToast={triggerToast} updateSelectedEncoder={updateSelectedEncoder} onKnobLedChange={handleKnobLedChange} onResetKnobLeds={() => void handleResetKnobLeds()} profileColorStrip={activeProfile.colorStrip} onApplyKnobLedPalette={handleApplyKnobLedPalette} onApplyKnobLedsToAllPages={handleApplyKnobLedsToAllPages} knobLedEffect={activePage.knobLedEffect ?? "static"} onKnobLedEffectChange={(effect) => void handleKnobLedEffectChange(effect)} updateSelectedKey={updateSelectedKey} updateSelectedZone={updateSelectedZone} useSampleAnimation={useSampleAnimation} />
        </div>
        </CatalogProvider>
      )}

      {/* 3. SCREEN 02: DEVICE MANAGER & DIAGNOSTICS (Spec Section 5) */}
      {activeScreen === "devices" && (
        <DevicesScreen connectedDevices={connectedDevices} activeDevice={activeDevice} frameStats={frameStats} diagnostics={diagnostics} onRescan={handleRescanDevices} onSelectDevice={handleSelectDevice} onBrightness={handleBrightnessChange} onOpenWorkspace={() => setActiveScreen("workspace")} onRestartDeck={handleRestartDeck} encoders={activePage.encoders} onKnobLedChange={handleKnobLedChange} onResetKnobLeds={() => void handleResetKnobLeds()} ledBrightness={ledBrightness} onLedBrightness={(pct) => void handleLedBrightnessChange(pct)} ledFollowSleep={ledFollowSleep} onLedSleepMode={(follow) => void handleLedSleepModeChange(follow)} knobLedEffect={activePage.knobLedEffect ?? "static"} onKnobLedEffectChange={(effect) => void handleKnobLedEffectChange(effect)} />
      )}

      {/* 4. SCREEN 04: PLUGINS & LEGACY .sdPlugin BRIDGE (Spec Section 7) */}
      {activeScreen === "plugins" && (
        <PluginsScreen legacySummary={legacySummary} onImportLegacy={handleImportLegacyProfiles} plugins={plugins} onPluginsChange={setPlugins} notify={triggerToast} />
      )}

      {activeScreen === "macros" && (
        <MacroStudio
          macros={macros}
          profiles={profiles}
          selectedMacroId={selectedMacroId}
          selectedStepIdx={selectedMacroStepIdx}
          runReport={macroRunReport}
          activeKeyLabel={activeKey ? `key ${activeKey.index + 1}` : null}
          onSelectMacro={setSelectedMacroId}
          onSelectStep={setSelectedMacroStepIdx}
          onChange={persistMacros}
          onRun={async (macro) => {
            try {
              const next = macros.some((item) => item.id === macro.id)
                ? macros.map((item) => (item.id === macro.id ? macro : item)) : [macro, ...macros];
              persistMacros(next);
              if (!await saveQueues.Macros.flush()) return;
              const report = await deckforgeApi.executeMacroSequence(macro.id);
              setMacroRunReport(report);
              triggerToast(`${report.macroName} finished in ${report.totalElapsedMs}ms`,
                report.stepResults.some(step => step.status !== "Passed") ? "warning" : "success");
            } catch (error) {
              triggerToast(typeof error === "string" ? error : "Could not run the macro", "warning");
            }
          }}
          onStop={async () => {
            try { triggerToast(await deckforgeApi.stopMacroExecution(), "info"); }
            catch { triggerToast("Could not stop the running macro", "warning"); }
          }}
          onAssign={(macro) => {
            void assignMacroToKey(macro);
          }}
        />
      )}

      {/* 6. SCREEN 06: ICON STUDIO (Spec Section 9) */}
      {activeScreen === "icons" && (
        <IconStudio
          keys={activePage.keys.slice(0, activeDevice.keys.count)}
          columns={activeDevice.keys.cols}
          selectedIndex={activeKey.index}
          clip={clipFor(activeKey.media)}
          pixelWidth={activeDevice.keys.pixelWidth}
          pixelHeight={activeDevice.keys.pixelHeight}
          onSelectKey={(index) => {
            setSurface("key");
            setSurfaceIndex(index);
            setSelectedKeyIdx(index);
          }}
          onChange={updateSelectedKey}
          onUpload={() => fileRef.current?.click()}
          onSample={() => void useSampleAnimation("key", activeKey.index)}
          onClearMedia={() => assignMedia("key", activeKey.index, null)}
          onMediaPatch={(patch) => {
            if (!activeKey.media) return;
            assignMedia("key", activeKey.index, { ...activeKey.media, ...patch });
          }}
          onPaintPage={handleBatchApplyStyleToPage}
          onApply={() => void handleApplyToDevice()}
          onDropFile={(file) => void ingestFile(file, "key", activeKey.index)}
          onBrandPick={(file) => void ingestFile(file, "key", activeKey.index)}
        />
      )}

      {activeScreen === "themes" && (
        <ThemesScreen
          keys={activePage.keys.slice(0, activeDevice.keys.count)}
          columns={activeDevice.keys.cols}
          rows={activeDevice.keys.rows}
          pixelWidth={activeDevice.keys.pixelWidth}
          pixelHeight={activeDevice.keys.pixelHeight}
          selectedIndex={activeKey.index}
          onApplyFrames={applyKeyFrames}
          notify={triggerToast}
        />
      )}

      {/* GAME MODE: close what you don't need while playing */}
      {activeScreen === "gamemode" && <GameModeStudio notify={triggerToast} />}

      {/* 7. SCREEN 07: SETTINGS (Spec Section 10) */}
      {activeScreen === "settings" && (
        <SettingsScreen profiles={profiles} startupProfileId={startupProfileId} onStartupProfileChange={async id => { await deckforgeApi.saveStartupProfile(id); setStartupProfileId(id); }} onBackup={handleBackup} onRestore={handleRestoreBackup} onNavigate={setActiveScreen} onOpenGuide={() => setShowOnboarding(true)} />
      )}

      </div>
      </div>

      {/* First-Run Onboarding / Hardware Guide Modal (Spec Section 11.1) */}
      {showOnboarding && (
        <OnboardingModal devices={devices} onClose={() => setShowOnboarding(false)} onOpenDevices={() => { setShowOnboarding(false); setActiveScreen("devices"); }} />
      )}

      <WorkspaceStatusBar
        deviceId={activeDevice.id} deviceName={activeDevice.modelName}
        profileId={activeProfile.id} profileName={activeProfile.name} page={activePage}
        saved={savedLocally && !Object.values(saveStatuses).some(s => s.pending)}
        refresh={appliedRefresh} onNotify={triggerToast}
      />
    </div>
  );
};
