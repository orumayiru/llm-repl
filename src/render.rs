// src/render.rs
use termimad::{MadSkin, crossterm::style::{Color, Attribute}};
use crate::state::RenderTheme;

// --- Define Type Alias FIRST ---
type Rgb = (u8, u8, u8); // Define the alias for (u8, u8, u8) tuple

// --- Define Nord Theme Colors (RGB Tuples) SECOND ---
const NORD_BG_DARK: Rgb = (46, 52, 64);  // nord0
const NORD_BG_LIGHT: Rgb = (59, 66, 82); // nord1
const NORD_FG_SUBTLE: Rgb = (76, 86, 106); // nord3
const NORD_FG_DEFAULT: Rgb = (216, 222, 233); // nord4
const NORD_FG_BRIGHT: Rgb = (236, 239, 244); // nord6
const NORD_CYAN: Rgb = (136, 192, 208); // nord8
const NORD_BLUE: Rgb = (129, 161, 193); // nord9
const NORD_RED: Rgb = (191, 97, 106);  // nord11
const NORD_GREEN: Rgb = (163, 190, 140); // nord14

// --- Define ThemePalette Struct THIRD ---
// Uses the Rgb type alias defined above
#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub prompt_bracket: Rgb,
    pub prompt_separator: Rgb,
    pub prompt_provider: Rgb,
    pub prompt_model: Rgb,
    pub prompt_arrow: Rgb,
    pub error: Rgb,
    pub info: Rgb,
    pub success: Rgb,
    pub command_output_raw: Rgb,
}

// --- Define Palette Creation Functions FOURTH ---
// These functions use the constants defined above
pub fn get_nord_palette() -> ThemePalette {
    ThemePalette {
        prompt_bracket: NORD_FG_SUBTLE, // Use constant
        prompt_separator: NORD_FG_SUBTLE,
        prompt_provider: NORD_BLUE,
        prompt_model: NORD_CYAN,
        prompt_arrow: NORD_FG_SUBTLE,
        error: NORD_RED,
        info: NORD_FG_SUBTLE,
        success: NORD_GREEN,
        command_output_raw: NORD_FG_DEFAULT,
    }
}

pub fn get_default_palette() -> ThemePalette {
    // Define default RGB tuples directly or use more constants
    ThemePalette {
        prompt_bracket: (128, 128, 128),
        prompt_separator: (128, 128, 128),
        prompt_provider: (173, 216, 230),
        prompt_model: (173, 216, 230),
        prompt_arrow: (128, 128, 128),
        error: (200, 0, 0), // Slightly less harsh red
        info: (128, 128, 128),
        success: (0, 180, 0), // Slightly less harsh green
        command_output_raw: (220, 220, 220),
        
    }
}


// --- MadSkin Creation Functions FIFTH ---
// These functions use the constants defined above

pub fn create_nord_skin() -> MadSkin {
    let mut skin = MadSkin::default();

    // Convert RGB tuples to termimad::Color where needed
    let nord4_term = Color::Rgb { r: NORD_FG_DEFAULT.0, g: NORD_FG_DEFAULT.1, b: NORD_FG_DEFAULT.2 };
    let nord1_term = Color::Rgb { r: NORD_BG_LIGHT.0, g: NORD_BG_LIGHT.1, b: NORD_BG_LIGHT.2 };
    let nord3_term = Color::Rgb { r: NORD_FG_SUBTLE.0, g: NORD_FG_SUBTLE.1, b: NORD_FG_SUBTLE.2 };
    let nord6_term = Color::Rgb { r: NORD_FG_BRIGHT.0, g: NORD_FG_BRIGHT.1, b: NORD_FG_BRIGHT.2 };
    let nord8_term = Color::Rgb { r: NORD_CYAN.0, g: NORD_CYAN.1, b: NORD_CYAN.2 };
    let nord9_term = Color::Rgb { r: NORD_BLUE.0, g: NORD_BLUE.1, b: NORD_BLUE.2 };


    // Base text
    skin.paragraph.set_fg(nord4_term);
    skin.table.set_fg(nord4_term);

    // Inline code & Code blocks
    skin.inline_code.set_bg(nord1_term);
    skin.inline_code.set_fg(nord4_term);
    skin.code_block.set_bg(nord1_term);
    skin.code_block.set_fg(nord4_term);

    // Headers
    skin.headers[0].set_fg(nord9_term);
    skin.headers[0].add_attr(Attribute::Bold);
    skin.headers[1].set_fg(nord9_term);
    skin.headers[1].add_attr(Attribute::Bold);
    skin.headers[2].set_fg(nord8_term);
    skin.headers[2].add_attr(Attribute::Bold);
    skin.headers[3].set_fg(nord8_term);
    skin.headers[4].set_fg(nord3_term);
    for header in &mut skin.headers {
        if header.compound_style.object_style.foreground_color.is_none() {
             header.compound_style.object_style.foreground_color = Some(nord4_term);
        }
    }

    // Bold / Italic
    skin.bold.add_attr(Attribute::Bold);
    skin.bold.set_fg(nord6_term);
    skin.italic.add_attr(Attribute::Italic);

    // Links - Keep commented out

    // Lists - Default styling

    // Quotes - **REMOVED** direct skin.block_quote access. Rely on defaults.

    // Tables - **REMOVED** direct skin.table_header access. Rely on defaults + Markdown bold.

    // Horizontal Rule
    skin.horizontal_rule.set_fg(nord3_term);

    skin
}

// --- Keep placeholder functions ---
pub fn create_gruvbox_skin() -> MadSkin {
    println!("WARN: Gruvbox theme not fully implemented, using Nord.");
    create_nord_skin()
}
pub fn create_grayscale_skin() -> MadSkin {
    println!("WARN: Grayscale theme not fully implemented, using Nord.");
    create_nord_skin()
}

/// Selects and returns the appropriate skin AND palette based on the theme enum.
pub fn get_theme_resources(theme: RenderTheme) -> (MadSkin, ThemePalette) {
     match theme {
        RenderTheme::Nord => (create_nord_skin(), get_nord_palette()),
        RenderTheme::Gruvbox => (create_gruvbox_skin(), get_default_palette()), // Use default palette for WIP
        RenderTheme::Grayscale => (create_grayscale_skin(), get_default_palette()), // Use default palette for WIP
        RenderTheme::Default => (MadSkin::default(), get_default_palette()),
    }
}