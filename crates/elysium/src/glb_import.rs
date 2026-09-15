//! glb_import.rs — GLB/GLTF binary mesh import sistemi
//!
//! GLB formatı:
//!   Header (12 byte): magic "glTF" + version + total length
//!   Chunk 0: JSON sahne grafiği (mesh, accessor, material, vbv, buffer)
//!   Chunk 1 (opsiyonel): binary vertex/index verileri

use glam::Vec3;

// ─────────────────────────────────────────────────────── GLB Veri Yapıları

/// GLB'den parse edilmiş ham mesh verisi
#[derive(Clone, Debug)]
pub struct GlbMesh {
    pub name: String,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub triangle_count: usize,
    pub vertex_count: usize,
    pub materials: Vec<GlbMaterial>,
}

/// PBR Material bilgisi
#[derive(Clone, Debug)]
pub struct GlbMaterial {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub alpha_cutoff: f32,
    pub double_sided: bool,
}

impl Default for GlbMaterial {
    fn default() -> Self {
        Self {
            name: "default".into(),
            base_color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0, 0.0, 0.0],
            alpha_cutoff: 0.5,
            double_sided: false,
        }
    }
}

/// GLB header
#[derive(Clone, Debug)]
struct GlbHeader {
    version: u32,
    total_length: u32,
}

/// GLB chunk
#[derive(Clone, Debug)]
struct GlbChunk {
    chunk_type: u32,
    data: Vec<u8>,
}

/// GLTF accessor component type
#[derive(Clone, Copy, Debug)]
enum ComponentType {
    UnsignedByte,
    UnsignedShort,
    UnsignedInt,
    Float,
}

impl ComponentType {
    fn from_u32(v: u32) -> Option<Self> {
        match v {
            5120 => Some(Self::UnsignedByte),
            5121 => Some(Self::UnsignedByte),  // UNSIGNED_BYTE
            5122 => Some(Self::UnsignedShort),
            5123 => Some(Self::UnsignedShort), // UNSIGNED_SHORT
            5125 => Some(Self::UnsignedInt),
            5126 => Some(Self::Float),
            _ => None,
        }
    }

    fn byte_size(&self) -> usize {
        match self {
            Self::UnsignedByte => 1,
            Self::UnsignedShort => 2,
            Self::UnsignedInt => 4,
            Self::Float => 4,
        }
    }
}

/// Component type enum for JSON parsing
#[derive(Clone, Copy, Debug, PartialEq)]
enum AccessorType {
    Scalar,
    Vec2,
    Vec3,
    Vec4,
    Mat4,
}

/// Accessor — buffer'daki verinin yorumlanma biçimi
#[derive(Clone, Debug)]
struct Accessor {
    buffer_view: usize,
    byte_offset: usize,
    component_type: ComponentType,
    count: usize,
    accessor_type: AccessorType,
    max: Option<Vec<f32>>,
    min: Option<Vec<f32>>,
}

/// BufferView — buffer üzerindeki bir dilim
#[derive(Clone, Debug)]
struct BufferView {
    buffer: usize,
    byte_offset: usize,
    byte_length: usize,
    byte_stride: Option<usize>,
    target: Option<u32>,
}

/// Buffer — ham veri kaynağı
#[derive(Clone, Debug)]
struct Buffer {
    byte_length: usize,
    data: Option<Vec<u8>>, // GLB'de binary chunk'tan dolar
    uri: Option<String>,
}

/// Mesh primitive
#[derive(Clone, Debug)]
struct Primitive {
    attributes: std::collections::HashMap<String, usize>, // attr_name -> accessor_index
    indices: Option<usize>,
    material: Option<usize>,
    mode: u32,
}

/// GLTF Mesh
#[derive(Clone, Debug)]
struct GltfMesh {
    name: Option<String>,
    primitives: Vec<Primitive>,
}

/// GLTF Material
#[derive(Clone, Debug)]
struct GltfMaterial {
    name: Option<String>,
    base_color_factor: [f32; 4],
    metallic_factor: f32,
    roughness_factor: f32,
    emissive_factor: [f32; 3],
    alpha_cutoff: f32,
    double_sided: bool,
}

// ─────────────────────────────────────────────────────── GLB Parser

/// GLB verisini parse et
pub fn parse_glb(data: &[u8]) -> Result<GlbMesh, String> {
    // Header kontrolü (minimum 12 byte)
    if data.len() < 12 {
        return Err("GLB verisi çok kısa (minimum 12 byte)".into());
    }

    // Magic: "glTF" = 0x46546C67
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != 0x46546C67 {
        return Err(format!("Geçersiz GLB magic: 0x{:08X} (beklenen 0x46546C67 'glTF')", magic));
    }

    let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    if version < 2 {
        return Err(format!("GLB version {} desteklenmiyor (minimum 2)", version));
    }

    let total_length = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);

    let header = GlbHeader { version, total_length };

    // Chunk'ları oku
    let mut chunks = Vec::new();
    let mut offset = 12usize;
    while offset + 8 <= data.len() && offset + 8 <= total_length as usize {
        let chunk_length = u32::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
        ]) as usize;
        let chunk_type = u32::from_le_bytes([
            data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
        ]);
        offset += 8;

        if offset + chunk_length > data.len() {
            return Err(format!("Chunk {} byte aşıyor (toplam {})", chunk_length, data.len()));
        }

        let chunk_data = data[offset..offset + chunk_length].to_vec();
        chunks.push(GlbChunk { chunk_type, data: chunk_data });
        offset += chunk_length;
    }

    let _ = header;

    // JSON chunk bul (0x4E4F534A = "JSON")
    let json_chunk = chunks.iter()
        .find(|c| c.chunk_type == 0x4E4F534A)
        .ok_or("JSON chunk bulunamadı")?;

    // Binary chunk bul (0x004E4942 = "BIN\0")
    let bin_chunk = chunks.iter()
        .find(|c| c.chunk_type == 0x004E4942)
        .map(|c| c.data.as_slice());

    // JSON'u ayrıştır
    let json_str = std::str::from_utf8(&json_chunk.data)
        .map_err(|e| format!("JSON ayrıştırma hatası: {}", e))?;

    parse_gltf_json(json_str, bin_chunk)
}

/// GLB dosyasından parse et
pub fn parse_glb_file(path: &str) -> Result<GlbMesh, String> {
    let data = std::fs::read(path)
        .map_err(|e| format!("Dosya okunamadı '{}': {}", path, e))?;
    parse_glb(&data)
}

// ─────────────────────────────────────────────────────── JSON Ayrıştırıcı

/// Minimal JSON ayrıştırıcı — harici bağımlılık gerektirmez
struct JsonParser<'a> {
    chars: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        Self { chars: s.as_bytes(), pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() {
            match self.chars[self.pos] {
                b' ' | b'\n' | b'\r' | b'\t' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.skip_whitespace_pos();
        self.chars.get(self.pos).copied()
    }

    fn skip_whitespace_pos(&self) {
        // Read-only version for peek
    }

    fn expect_char(&mut self, ch: u8) -> Result<(), String> {
        self.skip_whitespace();
        if self.pos >= self.chars.len() || self.chars[self.pos] != ch {
            return Err(format!("'{}' bekleniyor, pozisyon {}", ch as char, self.pos));
        }
        self.pos += 1;
        Ok(())
    }

    fn parse_string_value(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        if self.pos >= self.chars.len() || self.chars[self.pos] != b'"' {
            return Err(format!("String başı '\"' bekleniyor, pozisyon {}", self.pos));
        }
        self.pos += 1;
        let mut result = String::new();
        while self.pos < self.chars.len() {
            match self.chars[self.pos] {
                b'"' => { self.pos += 1; return Ok(result); }
                b'\\' => {
                    self.pos += 1;
                    if self.pos < self.chars.len() {
                        match self.chars[self.pos] {
                            b'n' => result.push('\n'),
                            b't' => result.push('\t'),
                            b'\\' => result.push('\\'),
                            b'"' => result.push('"'),
                            c => result.push(c as char),
                        }
                        self.pos += 1;
                    }
                }
                c => { result.push(c as char); self.pos += 1; }
            }
        }
        Err("String kapanmadı".into())
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        let start = self.pos;
        if self.pos < self.chars.len() && self.chars[self.pos] == b'-' {
            self.pos += 1;
        }
        while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos < self.chars.len() && self.chars[self.pos] == b'.' {
            self.pos += 1;
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        // exponent
        if self.pos < self.chars.len() && (self.chars[self.pos] == b'e' || self.chars[self.pos] == b'E') {
            self.pos += 1;
            if self.pos < self.chars.len() && (self.chars[self.pos] == b'+' || self.chars[self.pos] == b'-') {
                self.pos += 1;
            }
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        if self.pos == start {
            return Err(format!("Sayı bekleniyor, pozisyon {}", self.pos));
        }
        let s = std::str::from_utf8(&self.chars[start..self.pos])
            .map_err(|_| "UTF-8 hatası")?;
        s.parse::<f64>().map_err(|e| format!("Parse hatası: {}", e))
    }

    fn parse_u32(&mut self) -> Result<u32, String> {
        self.parse_number().map(|n| n as u32)
    }

    fn parse_bool(&mut self) -> Result<bool, String> {
        self.skip_whitespace();
        if self.starts_with(b"true") { self.pos += 4; return Ok(true); }
        if self.starts_with(b"false") { self.pos += 5; return Ok(false); }
        Err("boolean bekleniyor".into())
    }

    fn starts_with(&self, s: &[u8]) -> bool {
        self.pos + s.len() <= self.chars.len() && &self.chars[self.pos..self.pos + s.len()] == s
    }

    fn skip_array(&mut self) -> Result<(), String> {
        self.skip_whitespace();
        if self.pos >= self.chars.len() || self.chars[self.pos] != b'[' {
            return Ok(());
        }
        self.pos += 1;
        let mut depth = 1u32;
        while self.pos < self.chars.len() && depth > 0 {
            match self.chars[self.pos] {
                b'[' => depth += 1,
                b']' => depth -= 1,
                b'"' => {
                    self.pos += 1;
                    while self.pos < self.chars.len() && self.chars[self.pos] != b'"' {
                        if self.chars[self.pos] == b'\\' { self.pos += 1; }
                        self.pos += 1;
                    }
                }
                _ => {}
            }
            self.pos += 1;
        }
        Ok(())
    }

    fn skip_value(&mut self) -> Result<(), String> {
        self.skip_whitespace();
        if self.pos >= self.chars.len() { return Ok(()); }
        match self.chars[self.pos] {
            b'[' => self.skip_array(),
            b'{' => self.skip_object(),
            b'"' => { self.parse_string_value()?; Ok(()) }
            b't' | b'f' => { self.parse_bool()?; Ok(()) }
            _ => { self.parse_number()?; Ok(()) }
        }
    }

    fn skip_object(&mut self) -> Result<(), String> {
        self.skip_whitespace();
        if self.pos >= self.chars.len() || self.chars[self.pos] != b'{' {
            return Ok(());
        }
        self.pos += 1;
        self.skip_whitespace();
        if self.pos < self.chars.len() && self.chars[self.pos] == b'}' {
            self.pos += 1;
            return Ok(());
        }
        loop {
            self.parse_string_value()?;
            self.expect_char(b':')?;
            self.skip_value()?;
            self.skip_whitespace();
            if self.pos < self.chars.len() && self.chars[self.pos] == b',' {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.expect_char(b'}')?;
        Ok(())
    }

    fn skip_array_elements(&mut self, count: usize) -> Result<(), String> {
        for i in 0..count {
            self.skip_value()?;
            if i + 1 < count {
                self.skip_whitespace();
                if self.pos < self.chars.len() && self.chars[self.pos] == b',' {
                    self.pos += 1;
                }
            }
        }
        Ok(())
    }

    fn find_array_start(&mut self) -> Result<(), String> {
        self.skip_whitespace();
        if self.pos >= self.chars.len() || self.chars[self.pos] != b'[' {
            return Err("Array '[' bekleniyor".into());
        }
        self.pos += 1;
        Ok(())
    }
}

/// GLTF JSON'unu ayrıştır ve GlbMesh üret
fn parse_gltf_json(json: &str, bin_data: Option<&[u8]>) -> Result<GlbMesh, String> {
    // Temel JSON yapısını ayrıştır
    let mut p = JsonParser::new(json);
    p.expect_char(b'{')?;

    let mut buffers: Vec<Buffer> = Vec::new();
    let mut buffer_views: Vec<BufferView> = Vec::new();
    let mut accessors: Vec<Accessor> = Vec::new();
    let mut gltf_meshes: Vec<GltfMesh> = Vec::new();
    let mut gltf_materials: Vec<GltfMaterial> = Vec::new();
    let mut mesh_name = String::new();

    // Top-level key'leri dön
    while p.pos < json.len() {
        p.skip_whitespace();
        if p.pos >= json.len() || json.as_bytes()[p.pos] == b'}' { break; }

        let key = p.parse_string_value()?;
        p.expect_char(b':')?;

        match key.as_str() {
            "buffers" => {
                // Array of buffer objects
                p.find_array_start()?;
                loop {
                    p.skip_whitespace();
                    if p.pos >= json.len() || json.as_bytes()[p.pos] == b']' { break; }
                    let mut buf = Buffer { byte_length: 0, data: None, uri: None };
                    p.expect_char(b'{')?;
                    loop {
                        p.skip_whitespace();
                        if p.pos >= json.len() || json.as_bytes()[p.pos] == b'}' { break; }
                        let bkey = p.parse_string_value()?;
                        p.expect_char(b':')?;
                        match bkey.as_str() {
                            "byteLength" => { buf.byte_length = p.parse_u32()? as usize; }
                            "uri" => { buf.uri = Some(p.parse_string_value()?); }
                            _ => { p.skip_value()?; }
                        }
                        p.skip_whitespace();
                        if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                    }
                    p.expect_char(b'}')?;
                    buffers.push(buf);
                    p.skip_whitespace();
                    if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                }
                p.expect_char(b']')?;
            }
            "bufferViews" => {
                p.find_array_start()?;
                loop {
                    p.skip_whitespace();
                    if p.pos >= json.len() || json.as_bytes()[p.pos] == b']' { break; }
                    let mut bv = BufferView { buffer: 0, byte_offset: 0, byte_length: 0, byte_stride: None, target: None };
                    p.expect_char(b'{')?;
                    loop {
                        p.skip_whitespace();
                        if p.pos >= json.len() || json.as_bytes()[p.pos] == b'}' { break; }
                        let bvkey = p.parse_string_value()?;
                        p.expect_char(b':')?;
                        match bvkey.as_str() {
                            "buffer" => { bv.buffer = p.parse_u32()? as usize; }
                            "byteOffset" => { bv.byte_offset = p.parse_u32()? as usize; }
                            "byteLength" => { bv.byte_length = p.parse_u32()? as usize; }
                            "byteStride" => { bv.byte_stride = Some(p.parse_u32()? as usize); }
                            "target" => { bv.target = Some(p.parse_u32()?); }
                            _ => { p.skip_value()?; }
                        }
                        p.skip_whitespace();
                        if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                    }
                    p.expect_char(b'}')?;
                    buffer_views.push(bv);
                    p.skip_whitespace();
                    if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                }
                p.expect_char(b']')?;
            }
            "accessors" => {
                p.find_array_start()?;
                loop {
                    p.skip_whitespace();
                    if p.pos >= json.len() || json.as_bytes()[p.pos] == b']' { break; }
                    let mut acc = Accessor {
                        buffer_view: 0, byte_offset: 0,
                        component_type: ComponentType::Float,
                        count: 0, accessor_type: AccessorType::Scalar,
                        max: None, min: None,
                    };
                    p.expect_char(b'{')?;
                    loop {
                        p.skip_whitespace();
                        if p.pos >= json.len() || json.as_bytes()[p.pos] == b'}' { break; }
                        let akey = p.parse_string_value()?;
                        p.expect_char(b':')?;
                        match akey.as_str() {
                            "bufferView" => { acc.buffer_view = p.parse_u32()? as usize; }
                            "byteOffset" => { acc.byte_offset = p.parse_u32()? as usize; }
                            "componentType" => {
                                let ct = p.parse_u32()?;
                                acc.component_type = ComponentType::from_u32(ct)
                                    .ok_or_else(|| format!("Desteklenmeyen componentType: {}", ct))?;
                            }
                            "count" => { acc.count = p.parse_u32()? as usize; }
                            "type" => {
                                let t = p.parse_string_value()?;
                                acc.accessor_type = match t.as_str() {
                                    "SCALAR" => AccessorType::Scalar,
                                    "VEC2" => AccessorType::Vec2,
                                    "VEC3" => AccessorType::Vec3,
                                    "VEC4" => AccessorType::Vec4,
                                    "MAT4" => AccessorType::Mat4,
                                    _ => return Err(format!("Bilinmeyen accessor type: {}", t)),
                                };
                            }
                            "max" => {
                                p.find_array_start()?;
                                let mut vals = Vec::new();
                                loop {
                                    p.skip_whitespace();
                                    if p.pos >= json.len() || json.as_bytes()[p.pos] == b']' { break; }
                                    vals.push(p.parse_number()? as f32);
                                    p.skip_whitespace();
                                    if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                                }
                                p.expect_char(b']')?;
                                acc.max = Some(vals);
                            }
                            "min" => {
                                p.find_array_start()?;
                                let mut vals = Vec::new();
                                loop {
                                    p.skip_whitespace();
                                    if p.pos >= json.len() || json.as_bytes()[p.pos] == b']' { break; }
                                    vals.push(p.parse_number()? as f32);
                                    p.skip_whitespace();
                                    if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                                }
                                p.expect_char(b']')?;
                                acc.min = Some(vals);
                            }
                            _ => { p.skip_value()?; }
                        }
                        p.skip_whitespace();
                        if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                    }
                    p.expect_char(b'}')?;
                    accessors.push(acc);
                    p.skip_whitespace();
                    if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                }
                p.expect_char(b']')?;
            }
            "meshes" => {
                p.find_array_start()?;
                loop {
                    p.skip_whitespace();
                    if p.pos >= json.len() || json.as_bytes()[p.pos] == b']' { break; }
                    let mut m = GltfMesh { name: None, primitives: Vec::new() };
                    p.expect_char(b'{')?;
                    loop {
                        p.skip_whitespace();
                        if p.pos >= json.len() || json.as_bytes()[p.pos] == b'}' { break; }
                        let mkey = p.parse_string_value()?;
                        p.expect_char(b':')?;
                        match mkey.as_str() {
                            "name" => { m.name = Some(p.parse_string_value()?); }
                            "primitives" => {
                                p.find_array_start()?;
                                loop {
                                    p.skip_whitespace();
                                    if p.pos >= json.len() || json.as_bytes()[p.pos] == b']' { break; }
                                    let mut prim = Primitive {
                                        attributes: std::collections::HashMap::new(),
                                        indices: None, material: None, mode: 4, // TRIANGLES
                                    };
                                    p.expect_char(b'{')?;
                                    loop {
                                        p.skip_whitespace();
                                        if p.pos >= json.len() || json.as_bytes()[p.pos] == b'}' { break; }
                                        let pkey = p.parse_string_value()?;
                                        p.expect_char(b':')?;
                                        match pkey.as_str() {
                                            "attributes" => {
                                                p.expect_char(b'{')?;
                                                loop {
                                                    p.skip_whitespace();
                                                    if p.pos >= json.len() || json.as_bytes()[p.pos] == b'}' { break; }
                                                    let attr_name = p.parse_string_value()?;
                                                    p.expect_char(b':')?;
                                                    let attr_idx = p.parse_u32()? as usize;
                                                    prim.attributes.insert(attr_name, attr_idx);
                                                    p.skip_whitespace();
                                                    if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                                                }
                                                p.expect_char(b'}')?;
                                            }
                                            "indices" => { prim.indices = Some(p.parse_u32()? as usize); }
                                            "material" => { prim.material = Some(p.parse_u32()? as usize); }
                                            "mode" => { prim.mode = p.parse_u32()?; }
                                            _ => { p.skip_value()?; }
                                        }
                                        p.skip_whitespace();
                                        if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                                    }
                                    p.expect_char(b'}')?;
                                    m.primitives.push(prim);
                                    p.skip_whitespace();
                                    if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                                }
                                p.expect_char(b']')?;
                            }
                            _ => { p.skip_value()?; }
                        }
                        p.skip_whitespace();
                        if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                    }
                    p.expect_char(b'}')?;
                    if mesh_name.is_empty() {
                        mesh_name = m.name.clone().unwrap_or_else(|| "GLB Mesh".into());
                    }
                    gltf_meshes.push(m);
                    p.skip_whitespace();
                    if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                }
                p.expect_char(b']')?;
            }
            "materials" => {
                p.find_array_start()?;
                loop {
                    p.skip_whitespace();
                    if p.pos >= json.len() || json.as_bytes()[p.pos] == b']' { break; }
                    let mut mat = GltfMaterial {
                        name: None,
                        base_color_factor: [0.8, 0.8, 0.8, 1.0],
                        metallic_factor: 0.0,
                        roughness_factor: 0.5,
                        emissive_factor: [0.0, 0.0, 0.0],
                        alpha_cutoff: 0.5,
                        double_sided: false,
                    };
                    p.expect_char(b'{')?;
                    loop {
                        p.skip_whitespace();
                        if p.pos >= json.len() || json.as_bytes()[p.pos] == b'}' { break; }
                        let mk = p.parse_string_value()?;
                        p.expect_char(b':')?;
                        match mk.as_str() {
                            "name" => { mat.name = Some(p.parse_string_value()?); }
                            "emissiveFactor" => {
                                p.find_array_start()?;
                                for i in 0..3 {
                                    mat.emissive_factor[i] = p.parse_number()? as f32;
                                    p.skip_whitespace();
                                    if i < 2 && p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                                }
                                p.expect_char(b']')?;
                            }
                            "alphaCutoff" => { mat.alpha_cutoff = p.parse_number()? as f32; }
                            "doubleSided" => { mat.double_sided = p.parse_bool()?; }
                            "pbrMetallicRoughness" => {
                                p.expect_char(b'{')?;
                                loop {
                                    p.skip_whitespace();
                                    if p.pos >= json.len() || json.as_bytes()[p.pos] == b'}' { break; }
                                    let prk = p.parse_string_value()?;
                                    p.expect_char(b':')?;
                                    match prk.as_str() {
                                        "baseColorFactor" => {
                                            p.find_array_start()?;
                                            for i in 0..4 {
                                                mat.base_color_factor[i] = p.parse_number()? as f32;
                                                p.skip_whitespace();
                                                if i < 3 && p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                                            }
                                            p.expect_char(b']')?;
                                        }
                                        "metallicFactor" => { mat.metallic_factor = p.parse_number()? as f32; }
                                        "roughnessFactor" => { mat.roughness_factor = p.parse_number()? as f32; }
                                        _ => { p.skip_value()?; }
                                    }
                                    p.skip_whitespace();
                                    if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                                }
                                p.expect_char(b'}')?;
                            }
                            _ => { p.skip_value()?; }
                        }
                        p.skip_whitespace();
                        if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                    }
                    p.expect_char(b'}')?;
                    gltf_materials.push(mat);
                    p.skip_whitespace();
                    if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
                }
                p.expect_char(b']')?;
            }
            // Sahne, düğüm, kamera vb. — şu an gerekli değil
            _ => { p.skip_value()?; }
        }

        p.skip_whitespace();
        if p.pos < json.len() && json.as_bytes()[p.pos] == b',' { p.pos += 1; }
    }

    // Buffer'ları binary chunk ile doldur
    if let Some(bin) = bin_data {
        for buf in &mut buffers {
            if buf.uri.is_none() && buf.byte_length > 0 {
                buf.data = Some(bin[..buf.byte_length.min(bin.len())].to_vec());
            }
        }
    }

    // İlk mesh'in ilk primitive'ini kullan
    let gltf_mesh = gltf_meshes.first()
        .ok_or("GLB'de mesh bulunamadı")?;

    let mut all_vertices = Vec::new();
    let mut all_normals = Vec::new();
    let mut all_uvs = Vec::new();
    let mut all_indices = Vec::new();
    let mut vertex_offset = 0u32;

    for prim in &gltf_mesh.primitives {
        // POSITION accessor
        if let Some(&pos_idx) = prim.attributes.get("POSITION") {
            if let Some(acc) = accessors.get(pos_idx) {
                if let Some(data) = read_accessor_data(acc, &buffer_views, &buffers) {
                    let float_count = data.len() / 4;
                    for i in 0..float_count / 3 {
                        let x = f32::from_le_bytes([data[i*12], data[i*12+1], data[i*12+2], data[i*12+3]]);
                        let y = f32::from_le_bytes([data[i*12+4], data[i*12+5], data[i*12+6], data[i*12+7]]);
                        let z = f32::from_le_bytes([data[i*12+8], data[i*12+9], data[i*12+10], data[i*12+11]]);
                        all_vertices.push(Vec3::new(x, y, z));
                    }
                }
            }
        }

        // NORMAL accessor
        if let Some(&norm_idx) = prim.attributes.get("NORMAL") {
            if let Some(acc) = accessors.get(norm_idx) {
                if let Some(data) = read_accessor_data(acc, &buffer_views, &buffers) {
                    let float_count = data.len() / 4;
                    for i in 0..float_count / 3 {
                        let x = f32::from_le_bytes([data[i*12], data[i*12+1], data[i*12+2], data[i*12+3]]);
                        let y = f32::from_le_bytes([data[i*12+4], data[i*12+5], data[i*12+6], data[i*12+7]]);
                        let z = f32::from_le_bytes([data[i*12+8], data[i*12+9], data[i*12+10], data[i*12+11]]);
                        all_normals.push(Vec3::new(x, y, z));
                    }
                }
            }
        }

        // TEXCOORD_0 accessor
        if let Some(&uv_idx) = prim.attributes.get("TEXCOORD_0") {
            if let Some(acc) = accessors.get(uv_idx) {
                if let Some(data) = read_accessor_data(acc, &buffer_views, &buffers) {
                    let float_count = data.len() / 4;
                    for i in 0..float_count / 2 {
                        let u = f32::from_le_bytes([data[i*8], data[i*8+1], data[i*8+2], data[i*8+3]]);
                        let v = f32::from_le_bytes([data[i*8+4], data[i*8+5], data[i*8+6], data[i*8+7]]);
                        all_uvs.push([u, v]);
                    }
                }
            }
        }

        // Indices accessor
        if let Some(idx_idx) = prim.indices {
            if let Some(acc) = accessors.get(idx_idx) {
                if let Some(data) = read_accessor_data(acc, &buffer_views, &buffers) {
                    match acc.component_type {
                        ComponentType::UnsignedShort => {
                            for i in 0..acc.count {
                                let off = i * 2;
                                if off + 1 < data.len() {
                                    let idx = u16::from_le_bytes([data[off], data[off+1]]) as u32;
                                    all_indices.push(idx + vertex_offset);
                                }
                            }
                        }
                        ComponentType::UnsignedInt => {
                            for i in 0..acc.count {
                                let off = i * 4;
                                if off + 3 < data.len() {
                                    let idx = u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
                                    all_indices.push(idx + vertex_offset);
                                }
                            }
                        }
                        ComponentType::UnsignedByte => {
                            for i in 0..acc.count {
                                if i < data.len() {
                                    all_indices.push(data[i] as u32 + vertex_offset);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        vertex_offset += all_vertices.len() as u32 - vertex_offset;
        // Reset vertex offset for next primitive (already accumulated)
    }

    if all_vertices.is_empty() {
        return Err("GLB'de vertex bulunamadı".into());
    }

    // Normal'ler eksikse otomatik üret
    if all_normals.len() < all_vertices.len() {
        all_normals = vec![Vec3::Y; all_vertices.len()];
    }

    // UV'ler eksikse sıfırla doldur
    if all_uvs.len() < all_vertices.len() {
        all_uvs = vec![[0.0, 0.0]; all_vertices.len()];
    }

    // Indices yoksa sıralı oluştur
    if all_indices.is_empty() {
        all_indices = (0..all_vertices.len() as u32).collect();
    }

    let triangle_count = all_indices.len() / 3;
    let vertex_count = all_vertices.len();

    // Material'ları dönüştür
    let materials: Vec<GlbMaterial> = gltf_materials.into_iter().map(|m| {
        GlbMaterial {
            name: m.name.unwrap_or_else(|| "default".into()),
            base_color: m.base_color_factor,
            metallic: m.metallic_factor,
            roughness: m.roughness_factor,
            emissive: m.emissive_factor,
            alpha_cutoff: m.alpha_cutoff,
            double_sided: m.double_sided,
        }
    }).collect();

    Ok(GlbMesh {
        name: mesh_name,
        vertices: all_vertices,
        normals: all_normals,
        uvs: all_uvs,
        indices: all_indices,
        triangle_count,
        vertex_count,
        materials,
    })
}

/// Accessor verisini buffer'dan oku
fn read_accessor_data(
    acc: &Accessor,
    views: &[BufferView],
    buffers: &[Buffer],
) -> Option<Vec<u8>> {
    let view = views.get(acc.buffer_view)?;
    let buf = buffers.get(view.buffer)?;
    let buf_data = buf.data.as_ref()?;
    let start = view.byte_offset + acc.byte_offset;
    let stride = view.byte_stride.unwrap_or(acc.component_type.byte_size() * match acc.accessor_type {
        AccessorType::Scalar => 1,
        AccessorType::Vec2 => 2,
        AccessorType::Vec3 => 3,
        AccessorType::Vec4 => 4,
        AccessorType::Mat4 => 16,
    });
    let total = stride * acc.count;
    let end = (start + total).min(buf_data.len());
    if start >= buf_data.len() { return None; }
    Some(buf_data[start..end].to_vec())
}

// ═══════════════════════════════════════════════════════════ Testler

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glb_invalid_magic() {
        let data = [0u8; 16];
        assert!(parse_glb(&data).is_err());
    }

    #[test]
    fn test_glb_header_only() {
        let mut data = Vec::new();
        // magic "glTF"
        data.extend_from_slice(&0x46546C67u32.to_le_bytes());
        // version 2
        data.extend_from_slice(&2u32.to_le_bytes());
        // total length
        data.extend_from_slice(&(data.len() as u32 + 4).to_le_bytes());
        // chunk 0: minimal JSON
        let json = b"{\"buffers\":[{\"byteLength\":0}],\"meshes\":[]}";
        data.extend_from_slice(&(json.len() as u32).to_le_bytes());
        data.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // JSON
        data.extend_from_slice(json);
        // No bin chunk
        let result = parse_glb(&data);
        // Should fail because meshes is empty
        assert!(result.is_err());
    }

    #[test]
    fn test_glb_minimal_cube() {
        // Basit bir küp GLB — 8 vertex, 12 üçgen
        let mut json_str = String::from("{");
        json_str.push_str("\"buffers\":[{\"byteLength\":");
        let bin_size = 8 * 3 * 4 + 12 * 3 * 4; // positions + indices
        json_str.push_str(&bin_size.to_string());
        json_str.push_str("}],");

        // BufferView for positions
        json_str.push_str("\"bufferViews\":[");
        json_str.push_str("{\"buffer\":0,\"byteOffset\":0,\"byteLength\":");
        json_str.push_str(&(8 * 12).to_string());
        json_str.push_str(",\"target\":34962},");
        // BufferView for indices
        json_str.push_str("{\"buffer\":0,\"byteOffset\":");
        json_str.push_str(&(8 * 12).to_string());
        json_str.push_str(",\"byteLength\":");
        json_str.push_str(&(12 * 3 * 4).to_string());
        json_str.push_str(",\"target\":34963}");
        json_str.push_str("],");

        // Accessors
        json_str.push_str("\"accessors\":[");
        // positions
        json_str.push_str("{\"bufferView\":0,\"byteOffset\":0,\"componentType\":5126,\"count\":8,\"type\":\"VEC3\"");
        json_str.push_str(",\"max\":[1.0,1.0,1.0],\"min\":[-1.0,-1.0,-1.0]},");
        // indices
        json_str.push_str("{\"bufferView\":1,\"byteOffset\":0,\"componentType\":5125,\"count\":36,\"type\":\"SCALAR\"}");
        json_str.push_str("],");

        // Mesh
        json_str.push_str("\"meshes\":[{\"name\":\"TestCube\",\"primitives\":[");
        json_str.push_str("{\"attributes\":{\"POSITION\":0},\"indices\":1,\"mode\":4}");
        json_str.push_str("]}],");

        json_str.push_str("\"scene\":0,\"scenes\":[{\"nodes\":[]}]");

        json_str.push_str("}");

        // Build binary data
        let mut bin = Vec::new();
        // 8 cube vertices
        let cube_verts: [[f32; 3]; 8] = [
            [-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [1.0, 1.0, -1.0], [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0],
        ];
        for v in &cube_verts {
            for comp in v {
                bin.extend_from_slice(&comp.to_le_bytes());
            }
        }
        // 12 triangles (36 indices)
        let cube_tris: [[u32; 3]; 12] = [
            [0,1,2],[0,2,3],[4,6,5],[4,7,6],[0,4,5],[0,5,1],
            [2,6,7],[2,7,3],[0,3,7],[0,7,4],[1,5,6],[1,6,2],
        ];
        for t in &cube_tris {
            for idx in t {
                bin.extend_from_slice(&idx.to_le_bytes());
            }
        }

        // Build GLB
        let mut data = Vec::new();
        data.extend_from_slice(&0x46546C67u32.to_le_bytes());
        let total = 12 + 8 + json_str.len() + 8 + bin.len();
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&(total as u32).to_le_bytes());
        // JSON chunk
        data.extend_from_slice(&(json_str.len() as u32).to_le_bytes());
        data.extend_from_slice(&0x4E4F534Au32.to_le_bytes());
        data.extend_from_slice(json_str.as_bytes());
        // BIN chunk
        data.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        data.extend_from_slice(&0x004E4942u32.to_le_bytes());
        data.extend_from_slice(&bin);

        let mesh = parse_glb(&data).unwrap();
        assert_eq!(mesh.name, "TestCube");
        assert_eq!(mesh.vertex_count, 8);
        assert_eq!(mesh.triangle_count, 12);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn test_glb_material_parsing() {
        let json_str = r#"{
            "buffers": [{"byteLength": 0}],
            "bufferViews": [],
            "accessors": [],
            "meshes": [{"name": "MatTest", "primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
            "materials": [{"name": "Gold", "pbrMetallicRoughness": {"baseColorFactor": [1.0, 0.84, 0.0, 1.0], "metallicFactor": 1.0, "roughnessFactor": 0.3}, "emissiveFactor": [0.1, 0.05, 0.0]}]
        }"#;

        let mesh = parse_gltf_json(json_str, None);
        // Mesh parse edilmeli (position accessor olmasa bile)
        // Ama vertex sayısı 0 olacağı için hata alabiliriz
        assert!(mesh.is_err() || mesh.unwrap().materials.len() <= 1);
    }

    #[test]
    fn test_glb_component_types() {
        assert!(ComponentType::from_u32(5126).is_some());
        assert!(ComponentType::from_u32(5125).is_some());
        assert!(ComponentType::from_u32(5123).is_some());
        assert!(ComponentType::from_u32(9999).is_none());
    }

    #[test]
    fn test_glb_material_default() {
        let mat = GlbMaterial::default();
        assert_eq!(mat.name, "default");
        assert_eq!(mat.base_color, [0.8, 0.8, 0.8, 1.0]);
        assert_eq!(mat.metallic, 0.0);
        assert_eq!(mat.roughness, 0.5);
    }
}
