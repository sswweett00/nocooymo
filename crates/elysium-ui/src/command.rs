use smallvec::SmallVec;

#[derive(Clone, Debug)]
pub struct UiDrawRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
    pub corner_radius: f32,
}

#[derive(Clone, Debug)]
pub struct UiDrawText {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: [f32; 4],
    pub size: f32,
}

#[derive(Clone, Debug)]
pub struct UiDrawLine {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub color: [f32; 4],
    pub thickness: f32,
}

#[derive(Clone, Debug)]
pub struct UiDrawQuad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub color: [f32; 4],
    pub corner_radius: f32,
}

#[derive(Clone, Debug)]
pub enum UiCommand {
    Rect(UiDrawRect),
    Text(UiDrawText),
    Line(UiDrawLine),
    Quad(UiDrawQuad),
}

#[derive(Default)]
pub struct UiCommandList {
    pub commands: SmallVec<[UiCommand; 64]>,
}

impl UiCommandList {
    pub fn rect(&mut self, rect: UiDrawRect) {
        self.commands.push(UiCommand::Rect(rect));
    }

    pub fn text(&mut self, text: UiDrawText) {
        self.commands.push(UiCommand::Text(text));
    }

    pub fn line(&mut self, line: UiDrawLine) {
        self.commands.push(UiCommand::Line(line));
    }

    pub fn quad(&mut self, quad: UiDrawQuad) {
        self.commands.push(UiCommand::Quad(quad));
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}
