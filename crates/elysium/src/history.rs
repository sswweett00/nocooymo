//! history.rs — Basit undo/redo yığını.
//! Her "commit" bir tam sahne anlık görüntüsüdür (küçük sahneler için yeterli).

use crate::renderer::Scene;

const MAX_HISTORY: usize = 64;

pub struct History {
    undo_stack: Vec<Scene>,
    redo_stack: Vec<Scene>,
}

impl History {
    pub fn new() -> Self {
        Self { undo_stack: Vec::new(), redo_stack: Vec::new() }
    }

    /// Değişiklikten ÖNCEKI durumu kaydet.
    pub fn push(&mut self, scene: &Scene) {
        self.undo_stack.push(scene.clone());
        if self.undo_stack.len() > MAX_HISTORY {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Geri al: önceki durumu döndür, mevcut durumu redo'ya koy.
    pub fn undo(&mut self, current: &mut Scene) -> bool {
        match self.undo_stack.pop() {
            Some(prev) => {
                self.redo_stack.push(current.clone());
                *current = prev;
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self, current: &mut Scene) -> bool {
        match self.redo_stack.pop() {
            Some(next) => {
                self.undo_stack.push(current.clone());
                *current = next;
                true
            }
            None => false,
        }
    }

    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }
}
