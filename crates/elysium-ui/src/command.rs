//! Komut sistemi — UI için ertelenmiş komut arabelleği.

use crate::types::{WidgetId, Widget, UiEvent};

/// UI komutu
#[derive(Clone, Debug)]
pub enum UiCommand {
    AddWidget(Widget),
    RemoveWidget(WidgetId),
    SetVisible { id: WidgetId, visible: bool },
    SetRect { id: WidgetId, rect: [i32; 4] },
    SetText { id: WidgetId, text: String },
    SetValue { id: WidgetId, value: f32 },
    SetBool { id: WidgetId, value: bool },
    Focus(WidgetId),
    Defocus,
    EmitEvent(UiEvent),
}

/// Komut arabelleği
pub struct CommandBuffer {
    commands: Vec<UiCommand>,
}

impl Default for CommandBuffer {
    fn default() -> Self { Self::new() }
}

impl CommandBuffer {
    pub fn new() -> Self { Self { commands: Vec::new() } }
    pub fn push(&mut self, cmd: UiCommand) { self.commands.push(cmd); }
    pub fn add_widget(&mut self, widget: Widget) { self.push(UiCommand::AddWidget(widget)); }
    pub fn remove_widget(&mut self, id: WidgetId) { self.push(UiCommand::RemoveWidget(id)); }
    pub fn set_visible(&mut self, id: WidgetId, visible: bool) { self.push(UiCommand::SetVisible { id, visible }); }
    pub fn set_text(&mut self, id: WidgetId, text: impl Into<String>) { self.push(UiCommand::SetText { id, text: text.into() }); }
    pub fn focus(&mut self, id: WidgetId) { self.push(UiCommand::Focus(id)); }
    pub fn emit_event(&mut self, event: UiEvent) { self.push(UiCommand::EmitEvent(event)); }
    pub fn drain(&mut self) -> Vec<UiCommand> { std::mem::take(&mut self.commands) }
    pub fn len(&self) -> usize { self.commands.len() }
    pub fn is_empty(&self) -> bool { self.commands.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{WidgetId, WidgetType, WidgetState};

    #[test]
    fn test_command_buffer() {
        let mut buf = CommandBuffer::new();
        assert!(buf.is_empty());
        let id = WidgetId(1);
        buf.add_widget(Widget { id, widget_type: WidgetType::Button, rect: [0,0,100,30], visible: true, interactive: true, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() });
        buf.set_text(id, "Hello");
        assert_eq!(buf.len(), 2);
        let cmds = buf.drain();
        assert_eq!(cmds.len(), 2);
    }
}
