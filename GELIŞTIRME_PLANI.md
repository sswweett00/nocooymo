# Elysium Motoru: Detaylı Geliştirme Planı

> **Not:** Bu plan, Mimari belgesinde belirtilen 40 özelliği hayata geçirmek için hazırlanmıştır. Toplam tahmini süre: 41 hafta (~10 ay)

---

## 📋 Proje Durumu Analizi

### Mevcut Durum
- **Mimari Belge**: Çok kapsamlı ve net (315 sayfa, 40 özellik)
- **Workspace**: 10 crate içeren Rust workspace yapısı
- **Ana Binary**: GPUI tabanlı çalışan bir editör arayüzü
- **Durum**: %30-40 oranında MVP seviyesinde - bazı crate'ler sadece modül iskelesi

### Eksik / Tamamlanması Gereken Alanlar
- 3D render: Sadece software renderer (wgpu/WGPU entegrasyonu gerekli)
- Fiziksel motor: XPBD çözücü eksik (stub)
- Asset pipeline: Ely format ve Kiln sistemi eksik
- Kinetic Flux: Graph IR ve codegen tam sürüm gerekli
- ML/AI: ONNX entegrasyonu eksik
- Network: Weave CRDT sistemi yok

---

## 🎯 Geliştirme Planı (Fazlar Halinde)

### FAZ 1: Çekirdek Altyapı (ECS + Scheduler) - 4-6 hafta
**Öncelik: Yüksek**

#### 1.1 ECS Chunk/Archetype Implementasyonu
- [x] 16 KiB chunk sayfa yöneticisi (`alloc/` içinde)
- [x] Sparse set tabanlı tag component sistemi
- [x] Component ekleme/çıkarma batch işlemi
- [x] Entity generation/validity kontrolü
- [x] Archetype geçişleri için memory migration

#### 1.2 Sistem Zamanlayıcısı
- [x] Bağımlılık grafiği oluşturma
- [x] Rayon work-stealing entegrasyonu
- [x] Sistem aşaması (Update, Render, PostUpdate) yönetimi
- [x] Paralel chunk iteration desteği

#### 1.3 Command Buffer
- [x] Deferred spawn/despawn komutları
- [x] Component komutu tamponu
- [x] Frame sonu uygulama mekanizması

---

### FAZ 2: Render Alt Sistemi (WGPU + Meshlet) - 6-8 hafta
**Öncelik: Yüksek**

#### 2.1 RHI (Render Hardware Interface)
- [x] `RenderBackend` trait implementasyonu
- [x] WGPU backend tam entegrasyonu (nalgebra eklendi)
- [ ] Bindless descriptor set yönetimi
- [ ] Pipeline cache sistemi
- [ ] GPU profiling entegrasyonu

#### 2.2 Virtual Geometry (Meshlet)
- [ ] Meshlet oluşturma (128 tri / 64 vert)
- [ ] BVH hiyerarşisi
- [ ] GPU-driven LOD seçimi (compute shader)

#### 2.3 Weyra Pipeline
- [ ] Visibility buffer sistemi
- [ ] Hi-Z (hierarchical depth) optimizasyonu
- [ ] Software rasterization fallback

#### 2.4 Aether Virtual Texturing
- [ ] Page tabanlı doku streaming
- [ ] Feedback buffer yönetimi
- [ ] Mipmap streaming optimizasyonu

---

### FAZ 3: Fiziksel Motor - 5-6 hafta
**Öncelik: Orta-Yüksek**

#### 3.1 Tremor XPBD Çözücüsü
- [x] Broad-phase: Grid + AABB tree
- [x] Narrow-phase: GJK/EPA + SAT
- [x] Jacobi tabanlı paralel çözüm
- [x] CCD (Continuous Collision Detection)
- [x] Vehicle simulation (borrow hataları düzeltildi)
- [x] Soft body simulation (borrow hataları düzeltildi)
- [x] Fluid simulation (borrow hataları düzeltildi)

#### 3.2 Kırılma Sistemi
- [!] Voronoi ön-parçalama (sonraki adım)
- [ ] Runtime kısıt aktivasyonu
- [ ] Particle system entegrasyonu

#### 3.3 Akışkan Simülasyonu
- [x] FLIP/PIC hibird grid sistemi (temel yapı)
- [ ] SPH compute shader uyarlaması

---

### FAZ 4: Asset Pipeline & Ely Format - 4-5 hafta
**Öncelik: Orta**

#### 4.1 Ely Archive (.ely)
- [ ] Magic "ELY\0" formatı
- [ ] rkyv serileştirme entegrasyonu
- [ ] Sıkıştırma (LZ4/ZSTD)
- [ ] Bellek dosya eşlemeleme (mmap) desteği

#### 4.2 Kiln Async Pipeline
- [x] Tokio tabanlı async iş parçacığı (stub)
- [ ] Meshlet build sırasında oluşturma
- [ ] İlerleme takibi ve event sistemi
- [ ] Doku sıkıştırma (BC7/ASTC)

---

### FAZ 5: Kinetic Visual Scripting - 5-7 hafta
**Öncelik: Orta**

#### 5.1 IR (Intermediate Representation)
- [ ] SSA tabanlı KineticIR
- [ ] Type sistemi ve veri akışı analizi
- [ ] Yan etki yalıtım kontrolü

#### 5.2 Codegen
- [ ] Rust native kod üretimi
- [ ] Cranelift JIT entegrasyonu
- [ ] Otomatik SIMD vektörleştirme
- [ ] Shader varyant üretimi

#### 5.3 Editör Arayüzü
- [ ] Düğüm çizimi (GPUI tabanlı)
- [ ] Bağlama sistemi
- [ ] Tip uyumsuzluk kontrolü

---

### FAZ 6: Editor Geliştirme - 3-4 hafta
**Öncelik: Orta**

#### 6.1 Dock Layout Sistemi
- [ ] Panel kaydırma & küçültme
- [ ] Panel durum kaydetme
- [ ] Köşe yuvarlaklığı ve animasyonlar

#### 6.2 3D Viewport
- [ ] Görüş alma kamerası (orbit)
- [ ] Entity seçimi (ray-triangle pick)
- [ ] Gizmo transform: Translate/Rotate/Scale
- [ ] Snapping mekanizması

#### 6.3 Inspector
- [ ] Komponent property editörleri
- [ ] Custom widget desteği

#### 6.4 Undo/Redo
- [ ] Action trait sistemi
- [ ] Delta yığın yönetimi

---

### FAZ 7: Network & Weave - 3-4 hafta
**Öncelik: Düşük-Orta**

#### 7.1 CRDT Uygulaması
- [ ] List/Counter/YMap tipleri
- [ ] State çakışma çözümleme
- [ ] Yjs-benzeri protokol

#### 7.2 Network Layer
- [ ] WebRTC/UDP haberleşme
- [ ] Entity-level locking
- [ ] Permission service

---

### FAZ 8: ML/AI Entegrasyonu - 4-5 hafta
**Öncelik: Düşük**

#### 8.1 ONNX Runtime
- [ ] inference sistemi
- [ ] Rust bağlaması

#### 8.2 RL NPC Eğitimi
- [ ] Headless mod simülasyonu
- [ ] Reward fonksiyonları

#### 8.3 AI Animasyon
- [ ] Motion capture entegrasyonu
- [ ] Dudak senkronizasyonu

---

### FAZ 9: Platform & Dağıtım - 2-3 hafta
**Öncelik: Düşük**

#### 9.1 Cross-Compilation
- [ ] Windows/Mac/Linux hedefleri
- [ ] Cargo build scriptleri

#### 9.2 Store Entegrasyonu
- [ ] Steamworks/Epic API'ları
- [ ] Cloud gaming desteği (GFN/Xbox)

---

## 📊 Zaman Çizelgesi Tahmini

| Faz | Süre | Çalışan | Öncelik |
|-----|------|---------|---------|
| Faz 1 (ECS) | 6 hafta | 2 geliştirici | Yüksek |
| Faz 2 (Render) | 8 hafta | 2-3 geliştirici | Yüksek |
| Faz 3 (Physics) | 6 hafta | 1-2 geliştirici | Orta-Yüksek |
| Faz 4 (Asset) | 5 hafta | 1 geliştirici | Orta |
| Faz 5 (Kinetic) | 6 hafta | 1-2 geliştirici | Orta |
| Faz 6 (Editor) | 4 hafta | 1 geliştirici | Orta |
| Faz 7 (Network) | 4 hafta | 1 geliştirici | Düşük-Orta |
| Faz 8 (ML) | 5 hafta | 1 geliştirici | Düşük |
| Faz 9 (Platform) | 3 hafta | 1 geliştirici | Düşük |

**Toplam: 41 hafta (~10 ay) - 3-4 geliştirici ekibiyle**

---

## 🚀 MVP Hedefi (8-10 hafta)

1. Faz 1'in %70'i (Temel ECS + Scheduler)
2. Faz 2'in %50'si (WGPU temel render)
3. Faz 4'un %30'u (Basit asset import)
4. Faz 6'nın %40'ı (Editor UI temel panelleri)

**MVP ile çalışan bir editör:**
- ECS tabanlı varlık yönetimi
- WGPU ile 3D render
- Asset import pipeline
- Hierarchy/Inspector/Viewport panelleri

---

## ⚠️ Riskler & Dikkat Edilmesi Gerekenler

1. **WGPU shader derleme** - Naga entegrasyonu karmaşık
2. **XPBD paralelleştirme** - Rust'ta eşzamanlılık zor
3. **Meshlet build optimizasyonu** - Compute shader verimliliği kritik
4. **JIT güvenlik** - Cranelift memory management dikkat gerektirir

---

## 📁 Dosya Yapısı

```
nocooymo/
├── Cargo.toml              # Workspace manifest
├── GELIŞTIRME_PLANI.md    # Bu doküman
├── Mimari                  # Mimari tasarım
├── commands.md             # Build/run komutları
└── crates/
    ├── elysium-core/       # F01-F06: ECS, allocator, scheduler
    ├── elysium-render/     # F07-F10: RHI, meshlet, veyra, aether
    ├── elysium-physics/    # F11-F14: XPBD, fracture, fluids
    ├── elysium-flux/       # F15-F18: Visual scripting
    ├── elysium-asset/      # F19-F20: Ely format, Kiln pipeline
    ├── elysium-ui/         # F21-F24: Immediate mode UI
    ├── elysium-editor/     # F25-F30: Editor araçları
    ├── elysium-net/        # F31-F33: CRDT, network
    ├── elysium-ml/         # F34-F37: ML inference, animasyon
    └── elysium/            # Ana binary
```

---

## 🔄 İlerleme Takibi

Her faz için haftalık:
- Kapanan görevler: [x]
- Devam eden görevler: [-]  
- Yeni keşfedilen sorunlar: [!]