//! UI sisteminin kullanımını gösteren örnek

use elysium_ui::*;

fn main() {
    // UI sistemini başlat
    let mut ui_system = UiSystem::initialize_default();

    // Gerekli ayarlamaları yap
    println!("Elysium UI Sistemi başlatıldı");
    println!("Tema: {}", ui_system.theme.name);
    
    // Örnek bir widget ID oluşturma
    let widget_id = ui_system.create_widget_id("main_window", 0);
    println!("Widget ID oluşturuldu: {:?}", widget_id.as_u64());
    
    // Widget'ı odakla
    ui_system.focus_widget(widget_id);
    println!("Widget odaklandı: {:?}", ui_system.get_focused_widget().map(|id| id.as_u64()));
    
    // UI rect kullanımı
    let rect = UiRect::new(10.0, 10.0, 200.0, 100.0);
    println!("Dikdörtgen oluşturuldu: ({}, {})", rect.width, rect.height);
    
    let center = rect.center();
    println!("Merkez noktası: ({}, {})", center.x, center.y);
    
    // UI rect örnekleri
    let inset_rect = rect.inset(5.0);
    println!("İçerik dikdörtgeni: ({}, {}) - {}x{}", 
             inset_rect.x, inset_rect.y, inset_rect.width, inset_rect.height);
    
    // UI rect örnekleri
    let scaled_rect = rect.scale(1.5);
    println!("Ölçeklenmiş dikdörtgen: ({}, {}) - {}x{}", 
             scaled_rect.x, scaled_rect.y, scaled_rect.width, scaled_rect.height);
    
    // Widget durumu örneği
    let mut widget_state = WidgetState::new();
    widget_state.set_value_f32(0.75);
    widget_state.set_string("Merhaba Dünya".to_string());
    
    println!("Widget durumu güncellendi:");
    println!("  Float değeri: {}", widget_state.value_f32);
    println!("  Metin değeri: {}", widget_state.value_string);
    
    // Tema örneği
    let dark_theme = Theme::dark();
    println!("Koyu tema oluşturuldu: {}", dark_theme.name);
    
    let light_theme = Theme::light();
    println!("Açık tema oluşturuldu: {}", light_theme.name);
    
    // Tooltip örneği
    let mut tooltip = TooltipState::new();
    tooltip.show("Bu bir araç ipucudur", 100.0, 100.0);
    println!("Tooltip gösterildi: {} at {:?}", tooltip.text, tooltip.position);
    
    // Tab bar örneği
    let mut tab_bar = TabBarState::new();
    tab_bar.set_active_tab(0);
    println!("Aktif sekme: {:?}", tab_bar.active_tab);
    
    // Tree node örneği
    let mut tree_node = TreeNodeState::new(1);
    println!("Tree node derinliği: {}", tree_node.depth);
    
    // UI sistemini temizle
    ui_system.clear_focus();
    println!("Odaklama temizlendi: {:?}", ui_system.get_focused_widget());
}