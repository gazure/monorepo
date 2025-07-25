use dioxus::prelude::*;

use crate::Route;

// Components are automatically exported when marked as pub

/// Navigation link component with consistent styling
#[component]
pub fn NavLink(to: Route, children: Element) -> Element {
    rsx! {
        Link {
            to,
            class: "text-azure-600 hover:text-azure-800 font-medium transition-colors hover:text-shadow-azure",
            {children}
        }
    }
}

/// Primary button with azure gradient background
#[component]
pub fn PrimaryButton(to: Route, children: Element) -> Element {
    rsx! {
        Link {
            to,
            class: "px-6 py-3 bg-azure-gradient text-white rounded-lg shadow-lg hover:shadow-xl transform hover:-translate-y-0.5 transition-all duration-200",
            {children}
        }
    }
}

/// Secondary button with glass morphism effect
#[component]
pub fn SecondaryButton(to: Route, children: Element) -> Element {
    rsx! {
        Link {
            to,
            class: "px-6 py-3 glass-morphism text-azure-700 rounded-lg shadow-lg hover:shadow-xl transform hover:-translate-y-0.5 transition-all duration-200 hover:bg-white/10",
            {children}
        }
    }
}

/// Glass morphism card container
#[component]
pub fn GlassCard(children: Element, #[props(default = "")] class: &'static str) -> Element {
    let combined_class = if class.is_empty() {
        "glass-morphism p-8 rounded-xl".to_string()
    } else {
        format!("glass-morphism p-8 rounded-xl {class}")
    };

    rsx! {
        div { class: "{combined_class}", {children} }
    }
}

/// Floating orb background decoration
#[component]
pub fn FloatingOrb(size: &'static str, position: &'static str) -> Element {
    rsx! {
        div { class: "floating-orb {size} {position}" }
    }
}

/// Section heading with icon
#[component]
pub fn SectionHeading(icon: &'static str, text: &'static str) -> Element {
    rsx! {
        h2 { class: "text-3xl font-bold text-azure-800 mb-6 flex items-center justify-center gap-2",
            span { {icon} }
            span { {text} }
        }
    }
}

/// Large gradient heading
#[component]
pub fn GradientHeading(children: Element, #[props(default = "")] class: &'static str) -> Element {
    let combined_class = if class.is_empty() {
        "bg-gradient-to-r from-azure-600 to-ocean-deep bg-clip-text text-transparent".to_string()
    } else {
        format!(
            "bg-gradient-to-r from-azure-600 to-ocean-deep bg-clip-text text-transparent {class}"
        )
    };

    rsx! {
        h1 { class: "{combined_class}", {children} }
    }
}

/// Social link icon button
#[component]
pub fn SocialLink(href: &'static str, platform: &'static str) -> Element {
    rsx! {
        a {
            href,
            target: "_blank",
            rel: "noopener noreferrer",
            class: "group",
            div { class: "p-3 glass-morphism rounded-full group-hover:bg-white/20 transition-all duration-200",
                span { class: "text-azure-600 group-hover:text-azure-800", {platform} }
            }
        }
    }
}

/// Skill badge variants
#[derive(Clone, PartialEq)]
pub enum SkillVariant {
    Primary,
    Secondary,
    Glass,
}

/// Skill badge component
#[component]
pub fn SkillBadge(text: &'static str, variant: SkillVariant) -> Element {
    let variant_class = match variant {
        SkillVariant::Primary => "bg-azure-gradient text-white",
        SkillVariant::Secondary => "bg-ocean-gradient text-white",
        SkillVariant::Glass => "glass-morphism text-azure-700 hover:bg-white/20",
    };

    rsx! {
        div { class: "group",
            div { class: "{variant_class} p-4 rounded-lg text-center shadow-lg group-hover:shadow-xl transform group-hover:-translate-y-1 transition-all duration-200",
                span { class: "font-semibold", {text} }
            }
        }
    }
}

/// Page container with animated background
#[component]
pub fn PageContainer(children: Element, #[props(default = true)] show_orbs: bool) -> Element {
    rsx! {
        div { class: "min-h-screen py-20 px-4",
            if show_orbs {
                // Decorative background elements
                FloatingOrb { size: "w-72 h-72", position: "top-20 left-10" }
                FloatingOrb { size: "w-96 h-96", position: "bottom-10 right-20" }
                FloatingOrb { size: "w-64 h-64", position: "top-1/3 right-1/4" }
            }
            {children}
        }
    }
}

/// Center-aligned container with max width
#[component]
pub fn CenteredContainer(
    children: Element,
    #[props(default = "max-w-4xl")] max_width: &'static str,
) -> Element {
    rsx! {
        div { class: "{max_width} mx-auto", {children} }
    }
}
