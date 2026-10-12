"use strict";

/*
 * V6.2 controlled application boundary.
 * Authorization is enforced by Rust. Nova's current mind-state envelope may
 * preflight a proposed action, but only the trusted host can grant or execute it.
 * This file integrates that current state directly; it does not wrap an older
 * app-control implementation.
 */

const nova620AppControl = {
  grant: null,
  windows: [],
  busy: false
};

function nova620TauriInvokeAvailable() {
  return Boolean(window.__TAURI__?.core?.invoke);
}

async function nova620Invoke(command, args = {}) {
  if (!nova620TauriInvokeAvailable()) {
    throw new Error("Controlled app access is available only in the native Tauri build right now.");
  }
  return window.__TAURI__.core.invoke(command, args);
}

function nova620SetBridgeStatus(text) {
  if (elements.appBridgeStatus) elements.appBridgeStatus.textContent = text;
}

function nova620GrantTarget(grant) {
  return grant ? {
    kind: "window",
    windowId: String(grant.windowId || ""),
    pid: Number(grant.pid || 0),
    processName: grant.processName || "",
    adapter: grant.adapter || ""
  } : null;
}

function nova620SyncMindAuthority(grant) {
  if (!grant) {
    if (typeof v556ClearRuntimeAuthority === "function") v556ClearRuntimeAuthority("HOST_REPORTS_NO_GRANT");
    return null;
  }
  if (typeof v556AttachRuntimeAuthority !== "function") return null;
  return v556AttachRuntimeAuthority(grant);
}

function nova620CapabilityForAction(action) {
  if (["focus", "type_text", "type_and_enter", "press_enter"].includes(action)) return "approved_target.input";
  if (action === "capture_selection") return "approved_target.observe";
  return null;
}

async function nova620PreflightApprovedAction(action) {
  const grant = nova620AppControl.grant || await nova620RefreshStatus();
  if (!grant) throw new Error("No application is approved for this session.");
  nova620SyncMindAuthority(grant);
  const capability = nova620CapabilityForAction(action);
  if (!capability || typeof v556CheckRuntimeExternalAction !== "function") return { allowed: true, grant, capability };
  const decision = v556CheckRuntimeExternalAction({ capability, target: nova620GrantTarget(grant) });
  if (!decision.allowed) throw new Error(`Current mind-state scope denied this action: ${decision.reason}`);
  return { ...decision, grant, capability };
}

function nova620RenderGrant() {
  const grant = nova620AppControl.grant;
  if (!elements.appAccessStatus) return;

  if (!nova620TauriInvokeAvailable()) {
    elements.appAccessStatus.innerHTML = "<strong>Unavailable in browser mode.</strong><br>Run Nova through Tauri on Windows to approve an external application.";
    elements.appAccessButton?.classList.remove("app-access-live");
    return;
  }

  if (!grant) {
    elements.appAccessStatus.innerHTML = "<strong>No application access.</strong><br>Nova cannot control an external window until you approve one for this session.";
    elements.appAccessButton?.classList.remove("app-access-live");
    return;
  }

  elements.appAccessStatus.innerHTML = `
    <strong>Approved for this session</strong><br>
    ${escapeHtml(grant.title)}<br>
    <small>${escapeHtml(grant.processName)} • PID ${grant.pid} • ${escapeHtml(grant.adapter)}</small>
  `;
  elements.appAccessButton?.classList.add("app-access-live");
}

async function nova620RefreshStatus() {
  if (!nova620TauriInvokeAvailable()) {
    nova620AppControl.grant = null;
    nova620RenderGrant();
    return null;
  }
  try {
    nova620AppControl.grant = await nova620Invoke("app_control_status");
  } catch (error) {
    nova620AppControl.grant = null;
    nova620SetBridgeStatus(String(error));
  }
  nova620SyncMindAuthority(nova620AppControl.grant);
  nova620RenderGrant();
  return nova620AppControl.grant;
}

async function nova620RefreshWindows() {
  if (!nova620TauriInvokeAvailable()) {
    nova620RenderGrant();
    return [];
  }
  const adapter = elements.appAdapterSelect?.value || "chatgpt_browser";
  nova620SetBridgeStatus("Looking for visible application windows...");
  const windows = await nova620Invoke("app_control_list_windows");
  nova620AppControl.windows = windows;
  const shown = adapter === "chatgpt_browser"
    ? windows.filter(item => item.chatgptEligible)
    : windows;

  elements.appWindowSelect.innerHTML = "";
  for (const item of shown) {
    const option = document.createElement("option");
    option.value = item.windowId;
    option.textContent = `${item.processName || "unknown"} — ${item.title}`;
    elements.appWindowSelect.appendChild(option);
  }

  if (!shown.length) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = adapter === "chatgpt_browser"
      ? "No visible ChatGPT browser window found"
      : "No controllable visible windows found";
    elements.appWindowSelect.appendChild(option);
  }
  nova620SetBridgeStatus(`Found ${shown.length} eligible window${shown.length === 1 ? "" : "s"}.`);
  return shown;
}

async function nova620ApproveSelected() {
  const windowId = elements.appWindowSelect?.value;
  if (!windowId) throw new Error("Choose a visible application window first.");
  const adapter = elements.appAdapterSelect?.value || "chatgpt_browser";
  const selected = nova620AppControl.windows.find(item => item.windowId === windowId);
  const confirmed = confirm(
    `Approve Nova to control only this window for the current session?\n\n${selected?.processName || "Application"}\n${selected?.title || ""}\n\nAllowed: focus, type text, Enter, and capture text you explicitly select in that window.\n\nNot allowed: shell, launching processes, filesystem access, or switching to another app.`
  );
  if (!confirmed) return null;
  nova620AppControl.grant = await nova620Invoke("app_control_begin", { windowId, adapter });
  nova620SyncMindAuthority(nova620AppControl.grant);
  nova620RenderGrant();
  nova620SetBridgeStatus("Access approved. The Rust gatekeeper will reject actions outside this exact window/process.");
  return nova620AppControl.grant;
}

async function nova620EndAccess() {
  if (nova620TauriInvokeAvailable()) await nova620Invoke("app_control_end");
  nova620AppControl.grant = null;
  if (typeof v556ClearRuntimeAuthority === "function") v556ClearRuntimeAuthority("USER_REVOKED_HOST_AUTHORITY");
  nova620RenderGrant();
  nova620SetBridgeStatus("Application access ended.");
}

async function nova620FocusApproved() {
  await nova620PreflightApprovedAction("focus");
  await nova620Invoke("app_control_focus");
  nova620SetBridgeStatus("Approved application focused.");
}

function nova620LastAssistantMessage() {
  for (let i = state.messages.length - 1; i >= 0; i -= 1) {
    if (state.messages[i].role === "assistant") return state.messages[i];
  }
  return null;
}

async function nova620SendText(text, send = true) {
  const value = String(text || "").trim();
  if (!value) throw new Error("There is no text to send.");
  await nova620PreflightApprovedAction(send ? "type_and_enter" : "type_text");
  await nova620Invoke(send ? "app_control_type_and_enter" : "app_control_type_text", { text: value });
  nova620SetBridgeStatus(send ? "Sent text to the approved application and pressed Enter." : "Typed text into the approved application.");
}

async function nova620SendLastNovaReply() {
  const message = nova620LastAssistantMessage();
  if (!message) throw new Error("Nova has not produced a reply to send yet.");
  await nova620SendText(message.rawContent ?? message.content, true);
}

async function nova620CaptureSelectionIntoNova() {
  if (nova620AppControl.busy) return;
  nova620AppControl.busy = true;
  elements.captureAppSelectionButton.disabled = true;
  try {
    nova620SetBridgeStatus("Capturing selected text from the approved application...");
    await nova620PreflightApprovedAction("capture_selection");
    const captured = await nova620Invoke("app_control_capture_selection");
    const grant = nova620AppControl.grant || await nova620RefreshStatus();
    const sourceLabel = grant?.adapter === "chatgpt_browser" ? "ChatGPT" : (grant?.processName || "Approved app");
    nova620SetBridgeStatus(`Captured ${captured.length} characters from ${sourceLabel}. Nova is thinking...`);
    const reply = await handleUserMessage(captured, { sourceLabel });
    if (reply && elements.autoSendBridgeCheckbox?.checked) {
      await nova620SendText(reply, true);
      nova620SetBridgeStatus(`Captured ${sourceLabel}'s reply, let Nova respond, and sent Nova's new reply back.`);
    } else {
      nova620SetBridgeStatus(`Captured ${sourceLabel}'s reply and passed it to Nova.`);
    }
  } finally {
    nova620AppControl.busy = false;
    elements.captureAppSelectionButton.disabled = false;
  }
}

function nova620OpenAppAccess() {
  closeBrain();
  elements.appAccessDrawer?.classList.add("open");
  elements.drawerBackdrop?.classList.add("open");
  nova620RefreshStatus();
  nova620RefreshWindows().catch(error => nova620SetBridgeStatus(error.message || String(error)));
}

function nova620CloseAppAccess() {
  elements.appAccessDrawer?.classList.remove("open");
  if (!elements.brainDrawer?.classList.contains("open")) elements.drawerBackdrop?.classList.remove("open");
}

function nova620SafeAction(action) {
  return async () => {
    try { await action(); }
    catch (error) {
      console.error(error);
      nova620SetBridgeStatus(`Denied/failed: ${error.message || error}`);
    }
  };
}

async function nova620DispatchApprovedAppAction(proposal = {}) {
  const action = String(proposal.action || "");
  const preflight = await nova620PreflightApprovedAction(action);
  if (action === "focus") await nova620Invoke("app_control_focus");
  else if (action === "type_text") await nova620Invoke("app_control_type_text", { text: String(proposal.text || "") });
  else if (action === "type_and_enter") await nova620Invoke("app_control_type_and_enter", { text: String(proposal.text || "") });
  else if (action === "press_enter") await nova620Invoke("app_control_press_enter");
  else if (action === "capture_selection") {
    const text = await nova620Invoke("app_control_capture_selection");
    return { ok: true, action, capability: preflight.capability, text, preflight };
  } else {
    return { ok: false, reason: "UNSUPPORTED_HOST_ACTION", action };
  }
  return { ok: true, action, capability: preflight.capability, preflight };
}

globalThis.nova620DispatchApprovedAppAction = nova620DispatchApprovedAppAction;

if (elements.appAccessButton) elements.appAccessButton.addEventListener("click", nova620OpenAppAccess);
if (elements.closeAppAccessButton) elements.closeAppAccessButton.addEventListener("click", nova620CloseAppAccess);
if (elements.drawerBackdrop) elements.drawerBackdrop.addEventListener("click", nova620CloseAppAccess);
if (elements.refreshAppWindowsButton) elements.refreshAppWindowsButton.addEventListener("click", nova620SafeAction(nova620RefreshWindows));
if (elements.appAdapterSelect) elements.appAdapterSelect.addEventListener("change", nova620SafeAction(nova620RefreshWindows));
if (elements.enableAppAccessButton) elements.enableAppAccessButton.addEventListener("click", nova620SafeAction(nova620ApproveSelected));
if (elements.endAppAccessButton) elements.endAppAccessButton.addEventListener("click", nova620SafeAction(nova620EndAccess));
if (elements.focusApprovedAppButton) elements.focusApprovedAppButton.addEventListener("click", nova620SafeAction(nova620FocusApproved));
if (elements.sendLastNovaReplyButton) elements.sendLastNovaReplyButton.addEventListener("click", nova620SafeAction(nova620SendLastNovaReply));
if (elements.captureAppSelectionButton) elements.captureAppSelectionButton.addEventListener("click", nova620SafeAction(nova620CaptureSelectionIntoNova));

nova620RefreshStatus();
