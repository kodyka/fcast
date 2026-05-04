# Phase 28 — Chat Overlay & Moderation Placeholder

> **UI-only.** Render a Moblin-style chat surface (overlay + settings) so the
> design surface is complete. **No real chat protocol** (Twitch/Kick/YouTube
> IRC, TTS, bot rules, filters) is wired. Every list is driven by an inline
> `mock-*` model.

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7
**Functional integration:** Deferred — FCast has no chat protocol; this is a
visual placeholder only.

**Moblin source analogues** (21 files):
- `View/Stream/Overlay/StreamOverlayChatView.swift` — live overlay
- `View/Settings/Chat/{ChatSettingsView,ChatSettingsAppearanceView,ChatSettingsLayoutView,ChatBotSettingsView,ChatFiltersSettingsView,ChatNicknamesSettingsView,ChatTextToSpeechSettingsView}.swift`
- `View/ControlBar/QuickButton/Chat/{QuickButtonChatView,QuickButtonChatChatterInfoView,QuickButtonChatModerationView,QuickButtonChatUrlView}.swift`
- `View/Settings/Streams/Stream/Chat/StreamEmotesSettingsView.swift`
- `View/Settings/Talkback/TalkbackSettingsView.swift`
- `View/Utils/VoicesView.swift`
- `View/Settings/Scenes/Widgets/Widget/Alerts/{WidgetAlertsChatBotSettingsView,WidgetAlertsKickSettingsView,WidgetAlertsTwitchSettingsView,WidgetAlertsSpeechToTextSettingsView}.swift`
- `View/Settings/Scenes/Widgets/Widget/Chat/WidgetChatSettingsView.swift`

**Files to add:**
- `senders/android/ui/components/chat_message_row.slint`
- `senders/android/ui/components/chat_overlay.slint`
- `senders/android/ui/pages/chat_settings_page.slint`
- `senders/android/ui/pages/chat_bot_settings_page.slint`
- `senders/android/ui/pages/chat_filters_settings_page.slint`
- `senders/android/ui/pages/chat_nicknames_settings_page.slint`
- `senders/android/ui/pages/chat_tts_settings_page.slint`
- `senders/android/ui/pages/talkback_settings_page.slint`

---

## Goal

Surface a **chat-shaped UI** — message list, moderation context menu, settings
sub-pages — using only Slint stub data. The user has explicitly said *we want
UI without functionality*; this phase is the most extreme example because the
underlying chat protocol does not exist in FCast.

This phase serves as the **architectural skeleton** that lets the design
language stay consistent across "real" features and "speculative" features.

---

## Moblin source pattern (SwiftUI → Slint mapping)

Moblin's chat overlay is a `LazyVStack` reversed inside a `ScrollView`,
animating new messages from the bottom. Each row is a `HStack` of
`emotes + colored username + bold separator + message`.

```swift
// View/Stream/Overlay/StreamOverlayChatView.swift (excerpt)
ScrollViewReader { proxy in
    ScrollView {
        LazyVStack(alignment: .leading, spacing: 1) {
            ForEach(model.chatPostsBuffer.suffix(model.maximumPosts)) { post in
                HStack(alignment: .top) {
                    EmoteImagesView(emotes: post.emotes)
                    Text(post.username)
                        .foregroundColor(post.color ?? .white)
                        .bold()
                    Text(":").bold()
                    Text(post.message)
                }
            }
        }
    }
}
```

In Slint this becomes a `ListView` with a `chat-messages: [ChatMessage]`
inline stub, plus a `ChatMessageRow` component. The `LazyVStack` semantics
map directly onto `ListView`'s built-in virtualization.

---

## Tasks

### 28-A — `ChatMessage` struct in `bridge.slint`

- [ ] Add to `senders/android/ui/bridge.slint` (kept here to leave the door
  open for future Bridge promotion):

  ```slint
  export struct ChatMessage {
      id: int,
      author: string,
      author-color: color,
      text: string,
      timestamp: string,         // formatted "HH:MM"
      is-moderator: bool,
      is-streamer: bool,
      is-deleted: bool,
  }
  ```

- [ ] Do **not** add a `Bridge.chat-messages: [ChatMessage]` property — stub
  data lives on the consuming page.

### 28-B — `ChatMessageRow` component

- [ ] Create `senders/android/ui/components/chat_message_row.slint`:

  ```slint
  import { Theme } from "../theme.slint";
  import { ChatMessage } from "../bridge.slint";

  export component ChatMessageRow inherits Rectangle {
      in property <ChatMessage> message;
      in property <bool> show-timestamp: false;
      callback long-press();
      height: text.preferred-height + 8px;

      ta := TouchArea {
          long-press-duration: 500ms;
          pointer-event(ev) => {
              if (ev.kind == PointerEventKind.up && self.pressed-time > 500ms) {
                  root.long-press();
              }
          }
      }

      HorizontalLayout {
          padding-left: Theme.padding-screen;
          padding-right: Theme.padding-screen;
          spacing: 6px;

          if root.show-timestamp:
          Text {
              text: root.message.timestamp;
              color: Theme.text-secondary;
              font-family: "monospace";
              font-size: Theme.font-size-caption;
          }

          // Streamer / moderator badge
          if root.message.is-streamer || root.message.is-moderator:
          Rectangle {
              background: root.message.is-streamer ? Theme.accent : Theme.success;
              border-radius: 3px;
              width: 18px;
              height: 14px;
              y: 2px;
              Text {
                  text: root.message.is-streamer ? "S" : "M";
                  color: white;
                  font-size: 10px;
                  font-weight: FontWeight.bold;
                  horizontal-alignment: center;
                  vertical-alignment: center;
              }
          }

          Text {
              text: root.message.author;
              color: root.message.author-color;
              font-weight: FontWeight.bold;
              font-size: Theme.font-size-body;
          }

          text := Text {
              text: ": " + root.message.text;
              color: root.message.is-deleted ? Theme.text-disabled : Theme.text-primary;
              font-size: Theme.font-size-body;
              wrap: word-wrap;
              strikethrough: root.message.is-deleted;
              horizontal-stretch: 1;
          }
      }
  }
  ```

### 28-C — `ChatOverlay` component (live overlay)

- [ ] Create `senders/android/ui/components/chat_overlay.slint`:

  ```slint
  import { Theme } from "../theme.slint";
  import { ChatMessage } from "../bridge.slint";
  import { ChatMessageRow } from "chat_message_row.slint";

  export component ChatOverlay inherits Rectangle {
      // Inline stub — when wired, swap to Bridge.chat-messages
      in-out property <[ChatMessage]> mock-messages: [
          { id: 1, author: "viewer42", author-color: #6b7280,
            text: "First!", timestamp: "12:01", is-moderator: false,
            is-streamer: false, is-deleted: false },
          { id: 2, author: "ModBot",  author-color: #10b981,
            text: "Welcome to the cast",  timestamp: "12:02",
            is-moderator: true, is-streamer: false, is-deleted: false },
          { id: 3, author: "you", author-color: #3b82f6,
            text: "thanks!",       timestamp: "12:02",
            is-moderator: false, is-streamer: true, is-deleted: false },
          { id: 4, author: "spam-account", author-color: #6b7280,
            text: "[deleted]",     timestamp: "12:03",
            is-moderator: false, is-streamer: false, is-deleted: true },
      ];
      in property <bool> show-timestamps: false;

      background: Theme.surface-overlay;
      border-radius: Theme.radius-card;
      clip: true;

      ListView {
          for msg in root.mock-messages: ChatMessageRow {
              message: msg;
              show-timestamp: root.show-timestamps;
          }
      }
  }
  ```

- [ ] **Build check.**

### 28-D — `ChatSettingsPage`

- [ ] Add `Panel.chat-settings` to `bridge.slint`'s `Panel` enum
  (Phase 7-A). Add navigation row from `FullSettingsPage`:

  ```slint
  // In full_settings_page.slint
  SettingsValueRow {
      label: "Chat overlay";
      value: "Configure";
      clicked => { Bridge.active-panel = Panel.chat-settings; }
  }
  ```

- [ ] Create `senders/android/ui/pages/chat_settings_page.slint`:

  ```slint
  import { Theme } from "../theme.slint";
  import { Bridge, Panel } from "../bridge.slint";
  import { TextButton } from "../components/buttons.slint";
  import { SettingsToggleRow, SettingsValueRow, SettingsSection }
      from "../components/settings_rows.slint";

  export component ChatSettingsPage inherits Rectangle {
      in-out property <bool>   mock-enabled: true;
      in-out property <bool>   mock-bot-enabled: false;
      in-out property <bool>   mock-tts-enabled: false;
      in-out property <bool>   mock-show-deleted: false;
      in-out property <int>    mock-max-age: 60;
      in-out property <int>    mock-appearance-idx: 0;       // compact/normal/large
      in-out property <int>    mock-layout-idx: 0;           // bottom-up / top-down

      background: Theme.surface-primary;

      VerticalLayout {
          padding: Theme.padding-screen;
          spacing: Theme.spacing-md;

          HorizontalLayout {
              alignment: space-between;
              Text { text: "Chat"; color: Theme.text-primary;
                     font-size: Theme.font-size-title;
                     font-weight: FontWeight.semi-bold; }
              TextButton { label: "Done";
                  clicked => { Bridge.active-panel = Panel.none; } }
          }

          SettingsSection { title: "ENABLED";
              SettingsToggleRow { label: "Show chat";
                                  checked: root.mock-enabled;
                                  toggled => { root.mock-enabled = !root.mock-enabled; } }
          }

          SettingsSection { title: "APPEARANCE";
              SettingsValueRow { label: "Density"; value: ["Compact", "Normal", "Large"][root.mock-appearance-idx];
                  clicked => { root.mock-appearance-idx = mod(root.mock-appearance-idx + 1, 3); } }
              SettingsValueRow { label: "Layout";  value: ["Bottom up", "Top down"][root.mock-layout-idx];
                  clicked => { root.mock-layout-idx = mod(root.mock-layout-idx + 1, 2); } }
              SettingsValueRow { label: "Maximum message age (s)"; value: "\{root.mock-max-age}";
                  clicked => { root.mock-max-age = root.mock-max-age >= 600 ? 30 : root.mock-max-age + 30; } }
              SettingsToggleRow { label: "Show deleted messages";
                                  checked: root.mock-show-deleted;
                                  toggled => { root.mock-show-deleted = !root.mock-show-deleted; } }
          }

          SettingsSection { title: "AUTOMATION";
              SettingsValueRow { label: "Bot"; value: root.mock-bot-enabled ? "On" : "Off";
                  clicked => { Bridge.active-panel = Panel.chat-bot-settings; } }
              SettingsValueRow { label: "Filters"; value: "0 rules";
                  clicked => { Bridge.active-panel = Panel.chat-filters-settings; } }
              SettingsValueRow { label: "Nicknames"; value: "0 overrides";
                  clicked => { Bridge.active-panel = Panel.chat-nicknames-settings; } }
              SettingsValueRow { label: "Text-to-speech"; value: root.mock-tts-enabled ? "On" : "Off";
                  clicked => { Bridge.active-panel = Panel.chat-tts-settings; } }
          }
      }
  }
  ```

### 28-E — sub-pages

For each of `chat_bot_settings_page.slint`,
`chat_filters_settings_page.slint`, `chat_nicknames_settings_page.slint`,
`chat_tts_settings_page.slint`, `talkback_settings_page.slint`:

- [ ] Create the page with a header (title + Done button) and an inline
  `mock-*: [...]` list / toggles matching the Moblin SwiftUI shape.
  Examples below — adapt the same template per page.

  - **ChatBot:** toggle "Enabled" + 4 trigger rules using a `for`
    iteration on `mock-rules: [{ trigger: string, response: string }]`.
  - **Filters:** `ListView` of `mock-filters: [{ pattern: string, action: string }]`
    with action picker.
  - **Nicknames:** `ListView` of `mock-overrides: [{ user: string, nickname: string }]`.
  - **TTS:** voice picker (segmented, tied to `Utils/VoicesView.swift`'s
    voice list — represented as a `SettingsValueRow` cycler in the
    placeholder), rate/pitch sliders, "test voice" button (no-op).
  - **Talkback:** toggle, source picker (mic / system audio), latency
    slider, push-to-talk hold-to-talk picker.

### 28-F — `QuickButtonChatView` overlay panel

- [ ] When the user opens the chat quick-button (Phase 17 lets them add it),
  show a half-screen `Popup` with the live `ChatOverlay` plus a per-message
  long-press context menu:

  ```slint
  Popup {
      x: 0; y: parent.height / 2;
      width: parent.width; height: parent.height / 2;
      background: Theme.surface-overlay;
      VerticalLayout {
          ChatOverlay { mock-messages: <... shared stub ...>; }
          HorizontalLayout {
              spacing: 6px;
              TextButton { label: "Reply"; }
              TextButton { label: "Translate"; }
              TextButton { label: "Open URL"; }
              DestructiveButton { label: "Delete"; }
              DestructiveButton { label: "Ban"; }
          }
      }
  }
  ```

### 28-G — `WidgetAlerts*` placeholders

Moblin has 4 chat-flavored alert widgets (`Twitch`, `Kick`, `ChatBot`,
`SpeechToText`). For Phase 28 these are read-only summary cards on a single
`chat_alerts_settings_page.slint`:

- [ ] One section per platform with toggle + sample text. No real
  Twitch/Kick auth or alert-trigger config.

---

## Exit criteria

1. `chat_overlay.slint` renders 4 mock messages (regular / mod / streamer /
   deleted) with correct color coding, badge, and strikethrough.
2. Long-press on a message exposes a context-menu placeholder
   (no-op, but visible).
3. `ChatSettingsPage` renders all 4 sub-page navigation rows; each
   sub-page renders its mock list/toggles.
4. `cargo build -p android-sender` passes.
5. No `Bridge.chat-*` properties exist anywhere in the Slint tree.

---

## What's NOT in this phase

- Any chat protocol (Twitch IRC, Kick WebSocket, YouTube live chat, EventSub).
- Chat-bot rule-engine evaluation.
- TTS audio synthesis or voice list from the system.
- Emote rendering / 7TV / FFZ / BTTV integration.
- Translation / URL preview popups.
- Sending messages back to the receiver.
- The four "WidgetAlerts*" platform-specific event subscriptions
  (per-platform sub-streams of follows/raids/subs).

All of the above are parked in **Phase 8 (deferred)**.

---

## Slint best practices applied here

- **`ListView`** is used for the chat overlay because Moblin's
  `LazyVStack` semantics line up with Slint's built-in virtualization
  (per the [`ListView` reference](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx) — "Elements are only instantiated if they are visible").
- **`TouchArea` + `pointer-event` callback** with `pressed-time` is the
  Slint-1.15-compatible way to detect long-press without a separate
  gesture detector.
- **Inline `[...]` literal** for `mock-messages` keeps the surface
  reviewable in `slint-viewer` without any Rust code.
- **`strikethrough: bool` on `Text`** and `wrap: word-wrap` are the
  built-in Slint properties that handle the "deleted" + multi-line
  message rendering.
