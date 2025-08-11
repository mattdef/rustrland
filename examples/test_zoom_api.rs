use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing Hyprland Zoom API capabilities for magnify plugin...\n");
    
    // Get current cursor zoom settings
    println!("📋 Current Hyprland cursor zoom configuration:");
    let output = Command::new("hyprctl")
        .args(["keyword", "-r", "cursor_zoom_factor"])
        .output();
    
    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            println!("  cursor_zoom_factor: {}", stdout.trim());
            if !stderr.is_empty() {
                println!("  stderr: {}", stderr.trim());
            }
        }
        Err(e) => println!("❌ Failed to get cursor_zoom_factor: {}", e),
    }
    
    let output2 = Command::new("hyprctl")
        .args(["keyword", "-r", "cursor_zoom_rigid"])
        .output();
    
    match output2 {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            println!("  cursor_zoom_rigid: {}", stdout.trim());
            if !stderr.is_empty() {
                println!("  stderr: {}", stderr.trim());
            }
        }
        Err(e) => println!("❌ Failed to get cursor_zoom_rigid: {}", e),
    }
    
    println!();
    
    // Test setting zoom factor
    println!("🔍 Testing zoom factor changes:");
    
    // Test zoom in (2x)
    println!("  Setting zoom to 2.0x...");
    let zoom_result = Command::new("hyprctl")
        .args(["keyword", "cursor_zoom_factor", "2.0"])
        .output();
        
    match zoom_result {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            if !stdout.is_empty() {
                println!("    stdout: {}", stdout.trim());
            }
            if !stderr.is_empty() {
                println!("    stderr: {}", stderr.trim());
            }
            println!("    ✅ Zoom to 2.0x command executed");
        }
        Err(e) => println!("    ❌ Failed to set zoom: {}", e),
    }
    
    // Wait a moment
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Test zoom out (back to 1x)
    println!("  Setting zoom back to 1.0x...");
    let reset_result = Command::new("hyprctl")
        .args(["keyword", "cursor_zoom_factor", "1.0"])
        .output();
        
    match reset_result {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            if !stdout.is_empty() {
                println!("    stdout: {}", stdout.trim());
            }
            if !stderr.is_empty() {
                println!("    stderr: {}", stderr.trim());
            }
            println!("    ✅ Zoom reset to 1.0x command executed");
        }
        Err(e) => println!("    ❌ Failed to reset zoom: {}", e),
    }
    
    println!();
    println!("🎯 Hyprland zoom API test completed!");
    println!("   Note: Zoom effects should be visible on screen during test.");
    
    Ok(())
}