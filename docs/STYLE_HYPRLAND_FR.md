# Guide de Style des Fenêtres Hyprland

Ce document fournit des informations complètes sur le stylage des fenêtres dans Hyprland, à la fois via la configuration native et la bibliothèque Rust hyprland-rs.

## Table des Matières

- [Vue d'ensemble](#vue-densemble)
- [Règles de fenêtres Hyprland natives](#règles-de-fenêtres-hyprland-natives)
- [Syntaxe des règles de fenêtres](#syntaxe-des-règles-de-fenêtres)
- [Règles de style](#règles-de-style)
- [API hyprland-rs](#api-hyprland-rs)
- [Problèmes courants et solutions](#problèmes-courants-et-solutions)
- [Meilleures pratiques](#meilleures-pratiques)

## Vue d'ensemble

Hyprland offre plusieurs moyens de styliser les fenêtres :

1. **Style global** via la section `decoration` dans la configuration
2. **Style spécifique aux fenêtres** via `windowrule` et `windowrulev2`
3. **Style dynamique** via les commandes `hyprctl`
4. **Style programmatique** via la bibliothèque Rust hyprland-rs

## Règles de fenêtres Hyprland natives

### Types de règles de fenêtres

Hyprland supporte deux types de règles de fenêtres :

- **`windowrule`** (hérité) : Syntaxe simple avec correspondance limitée
- **`windowrulev2`** (recommandé) : Syntaxe avancée avec capacités de correspondance puissantes

### Syntaxe de base

```conf
# windowrule = RÈGLE, IDENTIFIANT_FENÊTRE
windowrule = opacity 0.8,^(kitty)$

# windowrulev2 = RÈGLE, PARAMÈTRES
windowrulev2 = opacity 0.8 0.6,class:^(kitty)$
```

⚠️ **Notes importantes :**
- Les règles de fenêtres sont **sensibles à la casse** (`firefox` ≠ `Firefox`)
- Les règles sont évaluées **de haut en bas** (l'ordre compte)
- Les motifs RegEx doivent **correspondre entièrement** aux valeurs de fenêtre (depuis v0.46.0)
- Hyprland utilise RE2 de Google pour l'analyse RegEx

## Syntaxe des règles de fenêtres

### Champs de correspondance de fenêtres

`windowrulev2` supporte ces paramètres de correspondance :

| Champ | Description | Type | Exemple |
|-------|-------------|------|---------|
| `class` | Classe de la fenêtre | RegEx | `class:^(firefox)$` |
| `title` | Titre de la fenêtre | RegEx | `title:^(.*YouTube.*)$` |
| `initialClass` | Classe initiale au lancement | RegEx | `initialClass:^(code)$` |
| `initialTitle` | Titre initial au lancement | RegEx | `initialTitle:^(New Tab)$` |
| `xwayland` | Statut XWayland | 0/1 | `xwayland:1` |
| `floating` | Statut flottant | 0/1 | `floating:1` |
| `fullscreen` | Statut plein écran | 0/1 | `fullscreen:1` |
| `pinned` | Statut épinglé | 0/1 | `pinned:1` |
| `focus` | Statut de focus | 0/1 | `focus:1` |
| `workspace` | ID/nom de l'espace de travail | ID/nom | `workspace:2` |
| `onworkspace` | Nombre de fenêtres sur l'espace | int | `onworkspace:>5` |

### Types de règles : Statique vs Dynamique

- **Règles statiques** : Évaluées une fois à l'ouverture de la fenêtre
- **Règles dynamiques** : Réévaluées quand une propriété correspondante change

Règles dynamiques courantes : `opacity`, `bordercolor`, `bordersize`

## Règles de style

### Style des bordures

#### Taille des bordures
```conf
# Définir la taille de bordure à 2px pour toutes les fenêtres flottantes
windowrulev2 = bordersize 2,floating:1

# Supprimer les bordures des fenêtres non focalisées
windowrulev2 = bordersize 0,focus:0

# Définir la taille de bordure selon la classe de fenêtre
windowrulev2 = bordersize 3,class:^(firefox)$
```

#### Couleur des bordures
```conf
# Couleur unique (s'applique aux deux active/inactive)
windowrulev2 = bordercolor rgb(FF0000),class:^(firefox)$

# Couleurs active et inactive
windowrulev2 = bordercolor rgb(00FF00) rgb(FF0000),focus:1

# RGBA avec transparence
windowrulev2 = bordercolor rgba(255,0,0,0.8) rgba(100,100,100,0.5),title:^(.*Terminal.*)$

# Format hexadécimal
windowrulev2 = bordercolor 0xFF00FF00 0x80FF0000,class:^(kitty)$
```

#### Couleurs de bordures dynamiques
```conf
# Bordure rouge en plein écran
windowrulev2 = bordercolor rgb(FF0000),fullscreen:1

# Bordure verte pour les fenêtres focalisées
windowrulev2 = bordercolor rgb(00FF00),focus:1

# Couleurs différentes par espace de travail
windowrulev2 = bordercolor rgb(0000FF),workspace:1
windowrulev2 = bordercolor rgb(00FF00),workspace:2
```

### Style des ombres

#### Règles d'ombres
```conf
# Désactiver les ombres pour des fenêtres spécifiques
windowrulev2 = noshadow,class:^(firefox)$

# Activer les ombres uniquement pour les fenêtres flottantes
windowrulev2 = shadow,floating:1
```

#### Configuration globale des ombres
```conf
decoration {
    drop_shadow = true
    shadow_range = 30
    shadow_render_power = 4
    shadow_offset = 0 5
    col.shadow = rgba(00000099)
    shadow_ignore_window = true
}
```

### Style d'opacité

#### Opacité de base
```conf
# Valeur d'opacité unique
windowrulev2 = opacity 0.8,class:^(kitty)$

# Opacité active et inactive
windowrulev2 = opacity 1.0 0.8,class:^(code)$

# Opacité active, inactive et plein écran
windowrulev2 = opacity 1.0 0.8 0.9,class:^(firefox)$
```

⚠️ **Notes importantes sur l'opacité :**
- Les valeurs d'opacité sont **multiplicatives** (0.5 × 0.5 = 0.25 total)
- Les valeurs > 1.0 peuvent causer des problèmes graphiques
- Utilisez `override` pour ignorer les paramètres d'opacité globaux :
  ```conf
  windowrulev2 = opacity 0.8 override,class:^(kitty)$
  ```

### Autres règles de style

#### Arrondi
```conf
windowrulev2 = rounding 10,class:^(kitty)$
windowrulev2 = rounding 0,fullscreen:1
```

#### Flou
```conf
windowrulev2 = noblur,class:^(firefox)$
windowrulev2 = blur,floating:1
```

#### Animation
```conf
windowrulev2 = animation popin,class:^(kitty)$
windowrulev2 = animation slide,workspace:special
```

## API hyprland-rs

### API Keyword

La bibliothèque hyprland-rs fournit l'API `Keyword` pour récupérer/définir les valeurs de configuration :

```rust
use hyprland::keyword::{Keyword, OptionValue};

// Récupérer une valeur de configuration
let border_size = Keyword::get("general:border_size")?;
match border_size.value {
    OptionValue::Int(size) => println!("Taille de bordure : {}", size),
    OptionValue::String(s) => println!("Taille de bordure : {}", s),
    OptionValue::Float(f) => println!("Taille de bordure : {}", f),
}

// Définir une valeur de configuration
Keyword::set("general:border_size", "2")?;
```

### Types OptionValue

L'enum `OptionValue` supporte trois variantes :

- `Int(i64)` : Entiers 64-bit
- `Float(f64)` : Nombres flottants 64-bit  
- `String(String)` : Valeurs de chaîne

### Gestion des couleurs

Les valeurs de couleur dans hyprland-rs sont généralement retournées sous forme de :

1. **Format chaîne** : `"rgba(255,0,0,255)"` ou `"rgb(255,0,0)"`
2. **Format entier** : Valeurs de couleur brutes comme `i64`
3. **Format personnalisé** : Définitions de couleurs complexes (dégradés, etc.)

#### Exemple de conversion de couleur

```rust
fn parse_color_from_hyprctl(output: &str) -> Option<String> {
    // Analyser le format "custom type: aa7c7674 0deg"
    for line in output.lines() {
        if line.contains("custom type:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in parts {
                if part.len() == 8 && part.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Some(hex_to_rgba(part));
                }
            }
        }
    }
    None
}

fn hex_to_rgba(hex: &str) -> String {
    if hex.len() == 8 {
        // Format : AARRGGBB
        if let Ok(color) = u32::from_str_radix(hex, 16) {
            let a = (color >> 24) & 0xFF;
            let r = (color >> 16) & 0xFF;
            let g = (color >> 8) & 0xFF;
            let b = color & 0xFF;
            return format!("rgba({}, {}, {}, {})", r, g, b, a);
        }
    }
    format!("rgba({})", hex)
}
```

### Limitations de récupération de configuration

Le `Keyword::get()` d'hyprland-rs a des limitations :

- **Valeurs de couleur** : Peut ne pas analyser correctement les formats de couleur complexes
- **Types personnalisés** : Les configurations avancées peuvent ne pas être accessibles
- **Fallback nécessaire** : Utiliser les commandes `hyprctl` pour une récupération de couleur fiable

#### Récupération de couleur fiable

```rust
async fn get_border_color_reliable() -> Result<String> {
    let output = tokio::process::Command::new("hyprctl")
        .arg("getoption")
        .arg("general:col.active_border")
        .output()
        .await?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    extract_color_from_output(&output_str)
}
```

## Problèmes courants et solutions

### 1. "Flash" de style lors de la création de fenêtre

**Problème** : La fenêtre apparaît avec le style par défaut, puis change vers le style correct

**Cause** : Style appliqué après la création de fenêtre via la commande `windowrulev2`

**Solution** : Appliquer le style pendant le spawn avec des règles de lancement

```rust
// Mauvais : Appliquer le style après le spawn
client.spawn_app("firefox").await?;
// La fenêtre apparaît avec le style par défaut
apply_window_rules(window_address).await?;

// Bon : Appliquer le style pendant le spawn
let spawn_rule = format!(
    "[float;bordersize 2;bordercolor {} {};pin] firefox",
    active_color, inactive_color
);
client.spawn_app(&spawn_rule).await?;
```

### 2. Récupération de couleur incohérente

**Problème** : `Keyword::get()` échoue à récupérer les valeurs de couleur

**Solution** : Utiliser la commande `hyprctl` comme fallback

```rust
async fn get_style_safe() -> HyprlandStyle {
    let mut style = HyprlandStyle::default();
    
    // Essayer l'API Keyword d'abord
    if let Ok(border) = Keyword::get("general:col.active_border") {
        // Gérer la récupération réussie
    } else {
        // Fallback vers hyprctl
        let output = Command::new("hyprctl")
            .arg("getoption")
            .arg("general:col.active_border")
            .output()
            .await?;
        style.active_border = parse_hyprctl_color(&output.stdout)?;
    }
    
    style
}
```

### 3. Problèmes de multiplication d'opacité

**Problème** : L'opacité devient trop faible due à la multiplication

**Solution** : Utiliser le flag `override` ou calculer soigneusement

```conf
# Problème : 0.5 (global) × 0.5 (règle) = 0.25 total
windowrulev2 = opacity 0.5,class:^(kitty)$

# Solution : Outrepasser l'opacité globale
windowrulev2 = opacity 0.8 override,class:^(kitty)$
```

### 4. Conflits de configuration d'ombre

**Problème** : Les ombres n'apparaissent pas comme attendu avec les bordures

**Cause** : Le rendu d'ombre a changé de l'intérieur des bordures vers l'extérieur

**Solution** : Configurer les ombres et bordures séparément

```conf
decoration {
    drop_shadow = true
    shadow_ignore_window = true  # Ignorer les règles d'ombre spécifiques aux fenêtres
    shadow_range = 20
    col.shadow = rgba(00000099)
}

# Règles spécifiques aux fenêtres séparées
windowrulev2 = bordersize 2,class:^(kitty)$
windowrulev2 = noshadow,class:^(no-shadow-app)$
```

### 5. Correspondance de motifs RegEx

**Problème** : Les règles de fenêtres ne s'appliquent pas à cause d'un RegEx incorrect

**Solution** : Utiliser des motifs précis et tester avec `hyprctl clients`

```bash
# Vérifier les propriétés de fenêtre
hyprctl clients | grep -A 5 "class:"

# Tester les motifs
windowrulev2 = bordercolor rgb(FF0000),class:^(exact-match)$
windowrulev2 = bordercolor rgb(00FF00),class:.*partial.*
windowrulev2 = bordercolor rgb(0000FF),negative:unwanted-class
```

## Meilleures pratiques

### 1. Organisation des règles

```conf
# Grouper les règles par objectif
# === RÈGLES D'OPACITÉ ===
windowrulev2 = opacity 0.9 0.7,class:^(code)$
windowrulev2 = opacity 1.0 override,class:^(firefox)$

# === RÈGLES DE BORDURE ===
windowrulev2 = bordercolor rgb(00FF00),focus:1
windowrulev2 = bordercolor rgb(555555),focus:0

# === RÈGLES FLOTTANTES ===
windowrulev2 = float,class:^(calculator)$
windowrulev2 = size 400 300,class:^(calculator)$
```

### 2. Considérations de performance

- Utiliser des **règles statiques** quand possible (évaluées une fois)
- Minimiser les **règles dynamiques** (réévaluées fréquemment)
- Grouper les règles liées pour la même fenêtre

### 3. Débogage des règles de fenêtres

```bash
# Obtenir les informations de fenêtre
hyprctl clients

# Tester la syntaxe des règles
hyprctl keyword windowrulev2 "bordercolor rgb(FF0000),class:^(test)$"

# Surveiller l'évaluation des règles
hyprctl monitors  # Vérifier les fenêtres actives
```

### 4. Cohérence du format de couleur

```rust
// Standardiser sur un format de couleur dans toute votre application
const ACTIVE_BORDER: &str = "rgba(124, 118, 116, 170)";
const INACTIVE_BORDER: &str = "rgba(204, 197, 195, 170)";

// Éviter de mélanger les formats
// ❌ Ne pas mélanger : "rgb(255,0,0)" et "0xFF0000FF"
// ✅ Utiliser de façon cohérente : "rgba(255, 0, 0, 255)"
```

### 5. Gestion de configuration

```rust
#[derive(Debug, Clone)]
pub struct WindowStyle {
    pub border_size: i32,
    pub active_border_color: String,
    pub inactive_border_color: String,
    pub shadow_enabled: bool,
    pub shadow_color: String,
    pub opacity: f32,
}

impl WindowStyle {
    pub fn to_spawn_rule(&self, position: (i32, i32), size: (i32, i32)) -> String {
        let shadow_rule = if self.shadow_enabled { "" } else { "noshadow;" };
        format!(
            "[float;bordersize {};bordercolor {} {};{}move {} {};size {} {}]",
            self.border_size,
            self.active_border_color,
            self.inactive_border_color,
            shadow_rule,
            position.0, position.1,
            size.0, size.1
        )
    }
}
```

## Conclusion

Hyprland fournit des capacités de stylage de fenêtres puissantes et flexibles à travers plusieurs interfaces. Pour les applications dynamiques comme Rustrland, combiner l'application de règles au moment du spawn avec des fallbacks hyprctl fournit l'expérience de stylage la plus fiable.

Points clés à retenir :
- Appliquer les styles au spawn de fenêtre pour éviter les artefacts visuels
- Utiliser les commandes hyprctl pour une récupération de configuration fiable
- Tester les motifs RegEx minutieusement
- Organiser les règles logiquement et documenter les configurations complexes
- Gérer la conversion de format de couleur de façon cohérente

Pour plus d'informations, se référer à :
- [Wiki Hyprland - Règles de fenêtres](https://wiki.hypr.land/Configuring/Window-Rules/)
- [Wiki Hyprland - Variables](https://wiki.hypr.land/Configuring/Variables/)  
- [Documentation hyprland-rs](https://docs.rs/hyprland/)