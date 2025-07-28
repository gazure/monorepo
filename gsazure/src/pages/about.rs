use dioxus::prelude::*;

use crate::components::{
    CenteredContainer, GlassCard, GradientHeading, PageContainer, SectionHeading, SkillBadge, SkillVariant,
};

/// About page
#[component]
pub fn About() -> Element {
    rsx! {
        PageContainer {
            GradientHeading { class: "text-5xl sm:text-6xl font-bold text-center mb-12", "About Me" }

            CenteredContainer {

                GlassCard { class: "text-center mb-12 p-6",
                    p { class: "text-xl text-azure-700",
                        "I write code sometimes and have a name with terrible SEO"
                    }
                }

                SectionHeading { icon: "✨", text: "Skills & Technologies" }

                div { class: "grid grid-cols-2 md:grid-cols-3 gap-4 mb-12",
                    SkillBadge { text: "Rust", variant: SkillVariant::Primary }
                    SkillBadge { text: "Dioxus", variant: SkillVariant::Primary }
                    SkillBadge { text: "PostgreSQL", variant: SkillVariant::Glass }
                    SkillBadge { text: "Docker", variant: SkillVariant::Secondary }
                    SkillBadge { text: "Golang", variant: SkillVariant::Primary }
                    SkillBadge {
                        text: "Distributed Systems",
                        variant: SkillVariant::Primary,
                    }
                    SkillBadge { text: "Python", variant: SkillVariant::Primary }
                    SkillBadge { text: "AWS", variant: SkillVariant::Primary }
                    SkillBadge { text: "Infrastructure", variant: SkillVariant::Glass }

                }

                SectionHeading { icon: "🚀", text: "Experience" }

                GlassCard { class: "mb-12 text-center",
                    p { class: "text-lg text-azure-700",
                        "Spent 9 running infrastructure for Tripit. Now building distributed systems at a starup."
                    }
                }
            }
        }
    }
}
