//! Gelişmiş UI özellikleri gösteren örnek

use elysium_ui::*;
use std::sync::Arc;
use std::time::Duration;

fn main() {
    // UI sistemini başlat
    let mut ui_system = UiSystem::initialize_default();
    
    println!("Elysium Gelişmiş UI Sistemi başlatıldı");
    println!("Tema: {}", ui_system.theme.name);
    
    // Responsive tasarım testi
    ui_system.update_responsive_ui(1920.0, 1080.0);
    println!("Responsive tasarım güncellendi: 1920x1080");
    
    // Animasyon testi
    let widget_id = ui_system.create_widget_id("animated_button", 0);
    println!("Animasyonlu widget oluşturuldu: {:?}", widget_id.as_u64());
    
    // Yeni layout sistemini test et
    let container_rect = UiRect::new(0.0, 0.0, 800.0, 600.0);
    let flex_layout = FlexLayout::new(LayoutDirection::Horizontal)
        .with_gap(10.0)
        .with_justify_content(JustifyContent::SpaceBetween)
        .with_align_items(AlignItems::Center);
    
    let constraints = vec![
        LayoutConstraints::new().with_min_width(100.0).with_max_width(200.0),
        LayoutConstraints::new().with_min_width(150.0).with_max_width(300.0),
        LayoutConstraints::new().with_min_width(120.0).with_max_height(50.0),
    ];
    
    let positions = flex_layout.calculate_positions(container_rect, 3, &constraints);
    println!("Flex layout ile {} pozisyon hesaplandı", positions.len());
    
    // Grid layout testi
    let grid_layout = GridLayout::new(3).with_gap(5.0);
    let grid_positions = grid_layout.calculate_grid_positions(UiRect::new(0.0, 0.0, 600.0, 400.0), 9);
    println!("Grid layout ile 9 eleman için pozisyonlar hesaplandı");
    
    // Dock layout testi
    let dock_layout = DockLayout::new(UiRect::new(0.0, 0.0, 1000.0, 800.0))
        .with_docked_area(DockArea::Top, 50.0)
        .with_docked_area(DockArea::Left, 200.0)
        .with_docked_area(DockArea::Right, 250.0);
    
    if let Some(top_area) = dock_layout.get_area(DockArea::Top) {
        println!("Dock area (Top): {}x{} @ {},{}", top_area.width, top_area.height, top_area.x, top_area.y);
    }
    
    // Widget durumu testi
    let mut widget_state = WidgetState::new();
    widget_state.set_value_f32(0.75);
    widget_state.set_string("Merhaba Gelişmiş UI".to_string());
    
    // Animasyon testi
    widget_state.update_animation(Duration::from_millis(100));
    println!("Widget animasyonu güncellendi, ilerleme: {:.2}", widget_state.animation_progress);
    
    // Drag & drop testi
    let drag_data = Arc::new("Sürükle bırak verisi".to_string());
    ui_system.start_drag_operation(widget_id, drag_data);
    println!("Drag & drop işlemi başlatıldı");
    
    // Slider testi
    let mut slider = SliderState::new(0.0, 100.0, 50.0);
    slider.set_value(75.0);
    println!("Slider değeri: {:.2} (normalize: {:.2})", slider.value, slider.normalized_value());
    
    // Progress bar testi
    let mut progress = ProgressBarState::new(0.0, 100.0);
    progress.set_value(65.0);
    println!("Progress değeri: {:.2}% (ilerleme: {:.2})", progress.value, progress.progress());
    
    // Modal testi
    let mut modal = ModalState::new("Bilgi");
    modal.content_size = (400.0, 300.0);
    println!("Modal oluşturuldu: {}", modal.title);
    
    // Menu testi
    let mut menu = MenuState::new();
    menu.add_item(MenuItem::new("Dosya"));
    menu.add_item(MenuItem::new("Düzenle"));
    menu.add_item(MenuItem::new("Yardım"));
    println!("Menü oluşturuldu, {} öğe var", menu.items.len());
    
    // Animasyon türleri testi
    let animations = [
        AnimationType::FadeIn,
        AnimationType::SlideIn(Direction::Up),
        AnimationType::Scale(1.2),
        AnimationType::Bounce,
        AnimationType::Pulse,
    ];
    
    println!("Animasyon türleri oluşturuldu:");
    for (i, anim) in animations.iter().enumerate() {
        println!("  {}: {:?}", i + 1, anim);
    }
    
    // Tema animasyon testi
    let fade_transition = ui_system.theme.get_transition_for_state("hover");
    println!("Hover geçiş süresi: {:?}", fade_transition.duration);
    
    let animation_duration = ui_system.theme.get_animation_duration("normal");
    println!("Normal animasyon süresi: {:?}", animation_duration);
    
    // Responsive state testi
    let responsive = ResponsiveState::new(1024.0, 768.0);
    println!("Responsive breakpoint: {:?}", responsive.current_breakpoint);
    println!("Kompakt UI kullan: {}", responsive.should_use_compact_ui());
    
    // UI rect işlemleri
    let mut rect = UiRect::new(100.0, 100.0, 200.0, 150.0);
    let offset_rect = rect.offset(50.0, 30.0);
    println!("Offset rect: {}x{} @ {},{}", offset_rect.width, offset_rect.height, offset_rect.x, offset_rect.y);
    
    let scaled_rect = rect.scale(1.5);
    println!("Scaled rect: {}x{} @ {},{}", scaled_rect.width, scaled_rect.height, scaled_rect.x, scaled_rect.y);
    
    // UI builder kullanımı
    // Bu örnek, UI builder'ın teorik kullanımını göstermektedir
    // Gerçek implementasyon için UI context gerekir
    
    println!("\nTüm gelişmiş UI özellikleri başarıyla test edildi!");
}