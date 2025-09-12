# 📋 Analyse de la Fonction Toggle Scratchpad

## 🌟 Vue d'ensemble

La fonctionnalité toggle scratchpad dans Rustrland offre un système sophistiqué de gestion des fenêtres permettant aux utilisateurs d'afficher/masquer des fenêtres d'application à la demande. Cette analyse examine l'implémentation, le flux de travail et les caractéristiques clés du mécanisme de basculement.

## 📊 Sommaire des Actions dans toggle_scratchpad()

| Étape | Action | Sous-fonction(s) impliquée(s) | Description |
|-------|--------|-------------------------------|-------------|
| 1 | ✅ Validation Configuration | `get_validated_config()` | Vérifie l'existence du scratchpad |
| 2 | 🔗 Connexion Hyprland | `get_hyprland_client()` | Établit la connexion API |
| 3 | 🔍 Vérification État | État interne | Recherche l'état du scratchpad |
| 4 | 🤔 Décision Action | Logique principale | Détermine masquer/afficher/spawn |
| 5 | ⚡ Exécution Action | `hide_scratchpad_window()`, `show_scratchpad_window()`, `spawn_and_show_scratchpad()` | Effectue l'action appropriée |
| 6 | 🛠️ Gestion Erreurs | Mécanismes de récupération | Traite les erreurs et récupère |

## 🔧 Implémentation Détaillée

### 🎯 Fonction Principale: toggle_scratchpad()

**📍 Emplacement**: `src/plugins/scratchpads.rs:1507`

```rust
async fn toggle_scratchpad(&mut self, name: &str) -> Result<String>
```

**👥 Responsabilités**:
- 🎭 Coordonne l'ensemble du processus de basculement
- 🧠 Gère la logique de décision principale
- 📞 Appelle les sous-fonctions appropriées selon l'état

### 1️⃣ Validation de la Configuration

**🔧 Sous-fonction appelée**: `get_validated_config(name)`

**👥 Qui fait quoi**:
- 🔍 Recherche le scratchpad dans la configuration TOML
- ✅ Valide que tous les paramètres requis sont présents
- 📦 Retourne une structure `ValidatedConfig` avec les détails du scratchpad

**⚙️ Comment**:
- 📂 Accède à `self.config.scratchpads.get(name)`
- 🔎 Vérifie les champs obligatoires (command, class)
- ⚡ Applique les valeurs par défaut pour les champs optionnels

### 2️⃣ Récupération du Client Hyprland

**🔧 Sous-fonction appelée**: `get_hyprland_client()`

**👥 Qui fait quoi**:
- 🌐 Établit une connexion avec le serveur Hyprland
- 🔌 Fournit une interface pour les appels API

**⚙️ Comment**:
- 📡 Utilise la crate `hyprland` pour créer un client
- 🔗 Gère la connexion réseau avec Hyprland
- 📤 Retourne un `Arc<hyprland::Client>` pour les opérations

### 3️⃣ Vérification de l'État Interne

**👥 Qui fait quoi**:
- 📊 Examine le `HashMap` interne `self.states`
- 🔎 Recherche l'état du scratchpad par nom
- 📈 Analyse l'état de spawn et les fenêtres

**⚙️ Comment**:
- 🗂️ Utilise `self.states.get(name)` pour récupérer `ScratchpadState`
- 🔢 Vérifie `state.is_spawned` et `state.windows.len()`
- 👁️ Détermine si des fenêtres sont visibles via `window_state.is_visible`

### 4️⃣ Décision d'Action

**🧠 Logique de décision**:

```
┌─────────────────┐
│ État existe ?   │
└─────┬───────────┘
      │ Oui
      ▼
┌─────────────────┐    Oui     ┌─────────────────┐
│ Fenêtres       ─┼─────────► │ Masquer fenêtre │
│ visibles ?      │           └─────────────────┘
└─────┬───────────┘
      │ Non
      ▼
┌─────────────────┐    Oui     ┌─────────────────┐
│ Spawnées ?     ─┼─────────► │ Afficher fenêtre│
└─────┬───────────┘           └─────────────────┘
      │ Non
      ▼
┌─────────────────┐
│ Spawn nouveau   │
└─────────────────┘
```

### 5️⃣ Exécution des Actions

#### 🙈 Masquage d'une Fenêtre Visible

**🔧 Sous-fonction**: `hide_scratchpad_window(client, hypr_window, name)`

**👥 Qui fait quoi**:
- 📤 Envoie des commandes Hyprland pour masquer la fenêtre
- 🔄 Met à jour l'état interne de visibilité
- 🎬 Gère les animations si configurées

**⚙️ Comment**:
- 🎯 Utilise `client.dispatch()` avec des commandes comme `movetoworkspace`
- ❌ Met à jour `window_state.is_visible = false`
- ⏱️ Applique les délais de masquage si configurés

#### 👁️ Affichage d'une Fenêtre Cachée

**🔧 Sous-fonction**: `show_scratchpad_window(client, hypr_window, validated_config, name)`

**👥 Qui fait quoi**:
- 📍 Positionne et dimensionne la fenêtre selon la configuration
- 🖥️ Rend la fenêtre visible sur l'espace de travail actuel
- 🔄 Met à jour l'état de visibilité

**⚙️ Comment**:
- 📐 Calcule la position basée sur `config.position` ou centrage automatique
- 📏 Applique la taille via `config.size`
- 🎯 Utilise `client.dispatch()` pour `movetoworkspace` et `focuswindow`

#### 🚀 Spawn et Affichage Nouveau

**🔧 Sous-fonction**: `spawn_and_show_scratchpad(name, validated_config)`

**👥 Qui fait quoi**:
- ▶️ Lance le processus de l'application
- ⏳ Attend que la fenêtre apparaisse
- ⚙️ Configure la fenêtre selon les paramètres

**⚙️ Comment**:
- 💻 Utilise `tokio::process::Command` pour exécuter `config.command`
- 👀 Surveille l'apparition de la fenêtre via `client.get_windows()`
- 🔧 Applique les propriétés (pinned, animation, etc.)

### 6️⃣ Gestion des Erreurs et Récupération

**🛠️ Mécanismes de récupération**:
- 🗑️ **Fenêtre disparue**: Nettoie l'état et spawn nouveau
- 🔧 **État corrompu**: Supprime l'entrée et recommence
- ⚠️ **Échec API**: Fallback avec messages d'erreur informatifs

**🔧 Sous-fonctions impliquées**:
- ✅ `mark_window_visible(name, address)`: Met à jour l'état de visibilité
- 🗑️ `states.remove(name)`: Nettoie l'état corrompu

## 🌟 Caractéristiques Clés

### 🧠 Gestion Intelligente de l'État

| Fonctionnalité | Description | Avantage |
|----------------|-------------|----------|
| 🧹 Nettoyage automatique | Suppression des références obsolètes | Mémoire optimisée |
| 🔄 Récupération d'état | Spawn nouvelles instances | Robustesse |
| 👁️ Suivi de visibilité | État précis des fenêtres | Précision |

### 🛡️ Gestion des Erreurs Robuste

- ✅ **Dégradation gracieuse**: Fallback vers spawn nouvelles instances
- 🔍 **Validation de fenêtre**: Vérification existence avant opérations
- 🔄 **Synchronisation d'état**: Mise à jour après opérations réussies

## 🔗 Intégration et Performance

### 🌐 Utilisation de l'API Hyprland

| Fonction API | Utilisation | Fréquence |
|--------------|-------------|-----------|
| `get_windows()` | État actuel des fenêtres | Fréquente |
| `get_current_workspace()` | Espace de travail actif | Par toggle |
| Commandes manipulation | Afficher/masquer fenêtres | Par action |

### ⚡ Considérations de Performance

```
Performance Metrics:
├── Recherche état: O(1) via HashMap
├── Appels API: Minimisés par cache
├── Nettoyage: Automatique des références
└── Synchronisation: Batch des mises à jour
```

## 🎉 Conclusion

La fonctionnalité toggle scratchpad démontre une approche robuste et pilotée par l'état pour la gestion des fenêtres, offrant un comportement de basculement fiable avec une gestion d'erreur complète et des optimisations de performance. Le système de suivi d'état interne permet un contrôle précis de la visibilité des fenêtres tout en maintenant la compatibilité avec les fonctionnalités de gestion des fenêtres de Hyprland.

---

*📝 Document généré automatiquement - Dernière mise à jour: $(date)*
