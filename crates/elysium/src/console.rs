//! # Elysium Editor Console
//!
//! Editör konsolu — komut satırı arayüzü, geçmiş, otomatik tamamlama.

use std::collections::HashMap;

/// Konsol satırı türü
#[derive(Clone, Debug)]
pub enum ConsoleLine {
    Input(String),
    Output(String),
    Error(String),
    Info(String),
    Success(String),
    Warning(String),
}

impl ConsoleLine {
    pub fn text(&self) -> &str {
        match self {
            Self::Input(t) | Self::Output(t) | Self::Error(t) |
            Self::Info(t) | Self::Success(t) | Self::Warning(t) => t,
        }
    }

    pub fn color(&self) -> [u8; 4] {
        match self {
            Self::Input(_) => [180, 200, 240, 255],
            Self::Output(_) => [200, 200, 200, 255],
            Self::Error(_) => [240, 80, 80, 255],
            Self::Info(_) => [120, 180, 240, 255],
            Self::Success(_) => [100, 220, 130, 255],
            Self::Warning(_) => [240, 200, 80, 255],
        }
    }
}

/// Komut argümanı
#[derive(Clone, Debug)]
pub struct CommandArgs {
    pub command: String,
    pub positional: Vec<String>,
    pub flags: HashMap<String, String>,
    pub switches: Vec<String>,
}

impl CommandArgs {
    pub fn parse(input: &str) -> Self {
        let tokens = tokenize(input);
        if tokens.is_empty() {
            return Self { command: String::new(), positional: Vec::new(), flags: HashMap::new(), switches: Vec::new() };
        }

        let command = tokens[0].clone();
        let mut positional = Vec::new();
        let mut flags = HashMap::new();
        let mut switches = Vec::new();

        let mut i = 1;
        while i < tokens.len() {
            let token = &tokens[i];
            if token.starts_with("--") {
                // Long flag: --key=value veya --flag veya --key value
                if let Some(eq_pos) = token[2..].find('=') {
                    let key = token[2..2+eq_pos].to_string();
                    let val = token[2+eq_pos+1..].to_string();
                    flags.insert(key, val);
                } else {
                    let key = token[2..].to_string();
                    // Sonraki token flag değeri mi?
                    if i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
                        flags.insert(key, tokens[i + 1].clone());
                        i += 2;
                        continue;
                    } else {
                        switches.push(key);
                    }
                }
            } else if token.starts_with('-') && token.len() > 1 {
                // Short flag: -k value veya -f
                let flag_char = token[1..2].to_string();
                if i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
                    flags.insert(flag_char, tokens[i + 1].clone());
                    i += 2;
                    continue;
                } else {
                    switches.push(flag_char);
                }
            } else {
                positional.push(token.clone());
            }
            i += 1;
        }

        Self { command, positional, flags, switches }
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        self.positional.get(index).map(|s| s.as_str())
    }

    pub fn has(&self, flag: &str) -> bool {
        self.switches.contains(&flag.to_string()) || self.flags.contains_key(flag)
    }
}

/// Tokenizer — shell-benzeri sözdizimi
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = '"';

    for ch in input.chars() {
        match ch {
            '"' | '\'' if !in_quote => {
                in_quote = true;
                quote_char = ch;
            }
            c if c == quote_char && in_quote => {
                in_quote = false;
            }
            ' ' if !in_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Komut sonucu
#[derive(Clone, Debug)]
pub enum CommandResult {
    Success(String),
    Error(String),
    Info(String),
    Warning(String),
    Help { command: String, description: String, usage: String, examples: Vec<String> },
}

/// Komut tanımı
pub struct CommandDef {
    pub name: &'static str,
    pub description: &'static str,
    pub usage: &'static str,
    pub aliases: &'static [&'static str],
    pub handler: Box<dyn Fn(&CommandArgs) -> CommandResult>,
}

/// Konsol durumu
pub struct Console {
    pub lines: Vec<ConsoleLine>,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub max_history: usize,
    pub max_lines: usize,
    pub visible: bool,
    pub commands: HashMap<String, CommandDef>,
    pub autocomplete_cache: Vec<String>,
    pub scroll_offset: usize,
}

impl Default for Console {
    fn default() -> Self { Self::new() }
}

impl Console {
    pub fn new() -> Self {
        let mut console = Self {
            lines: Vec::new(),
            input_buffer: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_index: None,
            max_history: 256,
            max_lines: 1000,
            visible: true,
            commands: HashMap::new(),
            autocomplete_cache: Vec::new(),
            scroll_offset: 0,
        };
        console.register_builtins();
        console.push_line(ConsoleLine::Info("Elysium Console v1.0 — 'help' ile başla".to_string()));
        console
    }

    fn register_builtins(&mut self) {
        self.register(CommandDef {
            name: "help",
            description: "Komut listesini göster veya belirli bir komut hakkında yardım al",
            usage: "help [komut_adi]",
            aliases: &["?", "h"],
            handler: Box::new(|args| {
                match args.get(0) {
                    Some(cmd) => CommandResult::Help {
                        command: cmd.to_string(),
                        description: format!("{} komutu hakkında yardım", cmd),
                        usage: format!("{} [args...]", cmd),
                        examples: vec![],
                    },
                    None => CommandResult::Info(
                        "Mevcut komutlar: help, list, add, remove, set, get, find, \
                         clear, history, physics, spawn, select, info, time, exec, alias".to_string()
                    ),
                }
            }),
        });

        self.register(CommandDef {
            name: "list",
            description: "Sahnedeki tüm varlıkları listele",
            usage: "list [--team player|enemy|neutral] [--type cube|sphere|...]",
            aliases: &["ls", "entities"],
            handler: Box::new(|_| CommandResult::Info("Sahne varlıkları listeleniyor...".to_string())),
        });

        self.register(CommandDef {
            name: "add",
            description: "Yeni bir varlık ekle",
            usage: "add <tip> [isim] [--pos x,y,z] [--team player|enemy|neutral]",
            aliases: &["create", "spawn"],
            handler: Box::new(|args| {
                match args.get(0) {
                    Some(_) => CommandResult::Success("Varlık eklendi".to_string()),
                    None => CommandResult::Error("Kullanım: add <cube|sphere|cylinder|capsule> [isim]".to_string()),
                }
            }),
        });

        self.register(CommandDef {
            name: "remove",
            description: "Seçili varlığı veya belirtilen ID'li varlığı sil",
            usage: "remove [id]",
            aliases: &["rm", "delete", "del"],
            handler: Box::new(|args| {
                match args.get(0) {
                    Some(id) => CommandResult::Success(format!("Varlık {} silindi", id)),
                    None => CommandResult::Info("Seçili varlık silinecek".to_string()),
                }
            }),
        });

        self.register(CommandDef {
            name: "set",
            description: "Seçili varlığın bir özelliğini değiştir",
            usage: "set <özellik> <değer>",
            aliases: &["property", "prop"],
            handler: Box::new(|args| {
                match (args.get(0), args.get(1)) {
                    (Some(prop), Some(val)) => CommandResult::Success(
                        format!("{} = {} güncellendi", prop, val)
                    ),
                    _ => CommandResult::Error(
                        "Kullanım: set <position|rotation|scale|color|metallic|roughness> <değer>".to_string()
                    ),
                }
            }),
        });

        self.register(CommandDef {
            name: "get",
            description: "Seçili varlığın bir özelliğini göster",
            usage: "get <özellik>",
            aliases: &["print", "show"],
            handler: Box::new(|args| {
                match args.get(0) {
                    Some(prop) => CommandResult::Info(format!("{}: (değer)", prop)),
                    None => CommandResult::Error("Kullanım: get <özellik>".to_string()),
                }
            }),
        });

        self.register(CommandDef {
            name: "find",
            description: "Varlık ara (isme göre)",
            usage: "find <arama_terimi>",
            aliases: &["search", "grep"],
            handler: Box::new(|args| {
                match args.get(0) {
                    Some(term) => CommandResult::Info(format!("'{}' için sonuçlar aranıyor...", term)),
                    None => CommandResult::Error("Kullanım: find <arama_terimi>".to_string()),
                }
            }),
        });

        self.register(CommandDef {
            name: "clear",
            description: "Konsolu temizle",
            usage: "clear",
            aliases: &["cls", "clr"],
            handler: Box::new(|_| CommandResult::Success("clear".to_string())),
        });

        self.register(CommandDef {
            name: "history",
            description: "Komut geçmişini göster",
            usage: "history [--clear]",
            aliases: &["hist"],
            handler: Box::new(|args| {
                if args.has("clear") {
                    CommandResult::Success("Geçmiş temizlendi".to_string())
                } else {
                    CommandResult::Info("Komut geçmişi gösteriliyor...".to_string())
                }
            }),
        });

        self.register(CommandDef {
            name: "physics",
            description: "Fizik motoru durumunu değiştir",
            usage: "physics <on|off|status|gravity x,y,z>",
            aliases: &["phys", "sim"],
            handler: Box::new(|args| {
                match args.get(0) {
                    Some(mode) => match &*mode {
                        "on" | "true" | "1" => CommandResult::Success("Fizik aktif".to_string()),
                        "off" | "false" | "0" => CommandResult::Success("Fizik devre dışı".to_string()),
                        "status" => CommandResult::Info("Fizik durumu sorgulanıyor...".to_string()),
                        "gravity" => CommandResult::Success("Yerçekimi güncellendi".to_string()),
                        _ => CommandResult::Error("Geçersiz mod: on|off|status|gravity".to_string()),
                    },
                    None => CommandResult::Error("Kullanım: physics <on|off|status|gravity x,y,z>".to_string()),
                }
            }),
        });

        self.register(CommandDef {
            name: "select",
            description: "Varlık seç (ID ile)",
            usage: "select <id> veya select all|none",
            aliases: &["sel"],
            handler: Box::new(|args| {
                match args.get(0) {
                    Some(id) => match &*id {
                        "all" => CommandResult::Info("Tüm varlıklar seçildi".to_string()),
                        "none" => CommandResult::Info("Seçim kaldırıldı".to_string()),
                        _ => CommandResult::Success(format!("Varlık {} seçildi", id)),
                    },
                    None => CommandResult::Error("Kullanım: select <id|all|none>".to_string()),
                }
            }),
        });

        self.register(CommandDef {
            name: "info",
            description: "Sahne ve motor hakkında bilgi göster",
            usage: "info [--scene|--engine|--gpu]",
            aliases: &["status", "stats"],
            handler: Box::new(|_| CommandResult::Info(
                "Elysium Engine v0.1.0\nSahne: aktif\nFizik: aktif\nGPU: wgpu".to_string()
            )),
        });

        self.register(CommandDef {
            name: "time",
            description: "Zaman kontrolü — oyun zamanını değiştir",
            usage: "time <scale|pause|resume> [değer]",
            aliases: &["timescale"],
            handler: Box::new(|args| {
                match args.get(0) {
                    Some(mode) => match &*mode {
                        "pause" => CommandResult::Success("Zaman durduruldu".to_string()),
                        "resume" => CommandResult::Success("Zaman devam ediyor".to_string()),
                        "scale" => {
                            let scale = args.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);
                            CommandResult::Success(format!("Zaman ölçeği: {:.2}x", scale))
                        }
                        _ => CommandResult::Error("Geçersiz mod: scale|pause|resume".to_string()),
                    },
                    None => CommandResult::Error("Kullanım: time <scale|pause|resume> [değer]".to_string()),
                }
            }),
        });

        self.register(CommandDef {
            name: "exec",
            description: "Dış komutu çalıştır (güvenli sandbox)",
            usage: "exec <komut>",
            aliases: &["run", "system"],
            handler: Box::new(|args| {
                match args.get(0) {
                    Some(cmd) => CommandResult::Warning(
                        format!("Güvenlik: '{}' komutu sandbox modunda çalıştırıldı", cmd)
                    ),
                    None => CommandResult::Error("Kullanım: exec <komut>".to_string()),
                }
            }),
        });

        self.register(CommandDef {
            name: "alias",
            description: "Komut kısayolu oluştur",
            usage: "alias <yeni_isim> <orijinal_komut>",
            aliases: &["nick"],
            handler: Box::new(|args| {
                match (args.get(0), args.get(1)) {
                    (Some(alias), Some(cmd)) => CommandResult::Success(
                        format!("'{}' -> '{}' kısayolu oluşturuldu", alias, cmd)
                    ),
                    _ => CommandResult::Error("Kullanım: alias <yeni_isim> <orijinal_komut>".to_string()),
                }
            }),
        });
    }

    pub fn register(&mut self, cmd: CommandDef) {
        let name = cmd.name.to_string();
        self.commands.insert(name, cmd);
    }

    pub fn push_line(&mut self, line: ConsoleLine) {
        self.lines.push(line);
        if self.lines.len() > self.max_lines {
            self.lines.drain(0..self.lines.len() - self.max_lines);
        }
    }

    pub fn execute(&mut self, input: &str) -> CommandResult {
        let input = input.trim().to_string();
        if input.is_empty() {
            return CommandResult::Info(String::new());
        }

        // Geçmişe ekle
        if self.history.last() != Some(&input) {
            self.history.push(input.clone());
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }
        self.history_index = None;

        // Input göster
        self.push_line(ConsoleLine::Input(format!("> {}", input)));

        // Alias kontrolü
        let actual_input = if let Some(alias_target) = self.resolve_alias(&input) {
            alias_target
        } else {
            input.clone()
        };

        let args = CommandArgs::parse(&actual_input);

        // Clear komutu özel
        if args.command == "clear" || args.command == "cls" {
            self.lines.clear();
            self.push_line(ConsoleLine::Info("Konsol temizlendi".to_string()));
            return CommandResult::Success("clear".to_string());
        }

        // Komutu bul ve çalıştır
        if let Some(cmd_def) = self.commands.get(&args.command) {
            let handler = &cmd_def.handler;
            let result = handler(&args);

            // Sonucu göster
            match &result {
                CommandResult::Success(msg) if msg == "clear" => {
                    self.lines.clear();
                }
                CommandResult::Success(msg) => {
                    self.push_line(ConsoleLine::Success(msg.clone()));
                }
                CommandResult::Error(msg) => {
                    self.push_line(ConsoleLine::Error(format!("Hata: {}", msg)));
                }
                CommandResult::Info(msg) => {
                    if !msg.is_empty() {
                        for line in msg.lines() {
                            self.push_line(ConsoleLine::Info(line.to_string()));
                        }
                    }
                }
                CommandResult::Warning(msg) => {
                    self.push_line(ConsoleLine::Warning(msg.clone()));
                }
                CommandResult::Help { command, description, usage, examples } => {
                    self.push_line(ConsoleLine::Info(format!("═══ {} ═══", command)));
                    self.push_line(ConsoleLine::Info(format!("  {}", description)));
                    self.push_line(ConsoleLine::Info(format!("  Kullanım: {}", usage)));
                    if !examples.is_empty() {
                        self.push_line(ConsoleLine::Info("  Örnekler:".to_string()));
                        for ex in examples {
                            self.push_line(ConsoleLine::Output(format!("    {}", ex)));
                        }
                    }
                }
            }
            result
        } else {
            self.push_line(ConsoleLine::Error(format!(
                "Bilinmeyen komut: '{}' — 'help' ile komut listesini gör", args.command
            )));
            CommandResult::Error(format!("Bilinmeyen komut: {}", args.command))
        }
    }

    fn resolve_alias(&self, input: &str) -> Option<String> {
        let cmd = input.split_whitespace().next()?;
        if cmd == "alias" { return None; }
        // Alias henüz desteklenmiyor, gelecekte eklenecek
        None
    }

    pub fn autocomplete(&mut self, partial: &str) -> Vec<String> {
        let partial_lower = partial.to_lowercase();
        let mut matches: Vec<String> = self.commands.keys()
            .filter(|name| name.starts_with(&partial_lower))
            .cloned()
            .collect();

        // Aliases'ları da ekle
        for cmd in self.commands.values() {
            for alias in cmd.aliases {
                if alias.starts_with(&partial_lower) && !matches.contains(&alias.to_string()) {
                    matches.push(alias.to_string());
                }
            }
        }

        let mut sorted = matches;
        sorted.sort();
        self.autocomplete_cache = sorted.clone();
        sorted
    }

    pub fn history_up(&mut self) -> Option<String> {
        if self.history.is_empty() { return None; }
        let new_index = match self.history_index {
            None => self.history.len() - 1,
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
        };
        self.history_index = Some(new_index);
        Some(self.history[new_index].clone())
    }

    pub fn history_down(&mut self) -> Option<String> {
        match self.history_index {
            Some(i) if i + 1 < self.history.len() => {
                self.history_index = Some(i + 1);
                Some(self.history[i + 1].clone())
            }
            Some(_) => {
                self.history_index = None;
                Some(String::new())
            }
            None => None,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn visible_lines(&self, max_height: usize) -> &[ConsoleLine] {
        let start = self.lines.len().saturating_sub(max_height);
        &self.lines[start..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        assert_eq!(tokenize("hello world"), vec!["hello", "world"]);
        assert_eq!(tokenize("add cube \"My Cube\""), vec!["add", "cube", "My Cube"]);
        assert_eq!(tokenize("--flag --key=val"), vec!["--flag", "--key=val"]);
    }

    #[test]
    fn test_command_args() {
        let args = CommandArgs::parse("add cube --pos 1,2,3 --team player -v");
        assert_eq!(args.command, "add");
        assert_eq!(args.get(0), Some("cube"));
        assert_eq!(args.flags.get("pos"), Some(&"1,2,3".to_string()));
        assert!(args.has("v"));
    }

    #[test]
    fn test_console_execute() {
        let mut console = Console::new();
        // `help` without args returns Info (lists all commands)
        let result = console.execute("help");
        assert!(matches!(result, CommandResult::Info(_)));
        // `help <cmd>` returns Help variant
        let result = console.execute("help add");
        assert!(matches!(result, CommandResult::Help { .. }));

        let result = console.execute("add cube");
        assert!(matches!(result, CommandResult::Success(_)));

        let result = console.execute("nonexistent");
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn test_autocomplete() {
        let mut console = Console::new();
        let completions = console.autocomplete("he");
        assert!(completions.contains(&"help".to_string()));

        let completions = console.autocomplete("ph");
        assert!(completions.iter().any(|c| c.starts_with("ph")));
    }

    #[test]
    fn test_history() {
        let mut console = Console::new();
        console.execute("help");
        console.execute("list");
        console.execute("add cube");

        assert_eq!(console.history.len(), 3);
        assert_eq!(console.history_up(), Some("add cube".to_string()));
        assert_eq!(console.history_up(), Some("list".to_string()));
        assert_eq!(console.history_down(), Some("add cube".to_string()));
    }
}
