//! Prompt templates and the `{{variable}}` layer.
//!
//! Prompts live in markdown files rather than in string literals so a user can
//! edit the app's voice without a rebuild (spec §9.2). The files are compiled
//! in as defaults so a fresh install has a working prompt with no files on
//! disk, and [`load_template`] is keyed by **name** — so the three templates
//! later sessions add (`evening-review`, `weekly-planning`, `monthly-review`)
//! are a file plus a line in [`BUILTIN_TEMPLATES`], with no code change here
//! or at any call site. [`super::context::SessionKind::template_name`] already
//! knows all four names.
//!
//! **An unknown variable is an error, not a literal.** A `{{typo}}` that
//! rendered through to the model would reach the user as a bad answer — the
//! model would try to interpret the braces, or quietly ignore the section that
//! was supposed to hold their commitments — and nothing in the chain would
//! report a fault. Failing at render time turns a prompt bug back into a
//! prompt bug.

use super::context::SessionContext;

/// The templates compiled into the binary.
///
/// Only the daily standup ships today. Adding another is one entry here and
/// one file beside it; nothing else in this module is aware of how many there
/// are.
pub const BUILTIN_TEMPLATES: &[(&str, &str)] = &[(
    "daily-standup",
    include_str!("../../prompts/daily-standup.md"),
)];

/// The variables a template may name.
///
/// Listed so an error can tell the author what was available, which is the
/// difference between a typo taking a minute and taking an afternoon.
pub const VARIABLES: &[&str] = &["date", "session", "commitment_map", "ranked_context"];

/// What can go wrong turning a template into a prompt.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PromptError {
    #[error("no prompt template named {name}")]
    UnknownTemplate { name: String },

    #[error("the template uses {{{{{name}}}}}, which is not a variable. Available: {available}")]
    UnknownVariable { name: String, available: String },

    #[error("the template has an unclosed {{{{ at byte {offset}")]
    UnclosedPlaceholder { offset: usize },
}

/// Loads a template by name.
///
/// Takes a name rather than a [`super::context::SessionKind`] so the set of
/// templates and the set of sessions can grow independently: a session with no
/// template of its own can borrow another's, and a template can exist before
/// the session that will use it.
pub fn load_template(name: &str) -> Result<&'static str, PromptError> {
    BUILTIN_TEMPLATES
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, body)| *body)
        .ok_or_else(|| PromptError::UnknownTemplate {
            name: name.to_string(),
        })
}

/// Substitutes `{{variable}}` throughout `template`.
///
/// Returns a `Result` where the spec sketched a bare `String`, for the reason
/// in this module's header: an unknown variable has to be a visible, testable
/// failure, and there is nowhere else in the chain for it to surface.
///
/// The scan is deliberately literal — no escaping, no expressions, no
/// conditionals. A template language would be one more thing between the user
/// and the model that could be wrong.
pub fn render_prompt(template: &str, ctx: &SessionContext) -> Result<String, PromptError> {
    let map = ctx.map.render();
    let items = ctx.render_context();

    let mut out = String::with_capacity(template.len() + map.len() + items.len());
    let mut rest = template;
    let mut consumed = 0usize;

    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);

        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err(PromptError::UnclosedPlaceholder {
                offset: consumed + start,
            });
        };

        let name = after[..end].trim();
        out.push_str(&value_of(name, ctx, &map, &items)?);

        consumed += start + 2 + end + 2;
        rest = &after[end + 2..];
    }

    out.push_str(rest);

    Ok(out)
}

/// Renders one session end to end: template by name, then substitution.
pub fn render_session(ctx: &SessionContext) -> Result<String, PromptError> {
    render_prompt(load_template(ctx.kind.template_name())?, ctx)
}

fn value_of(
    name: &str,
    ctx: &SessionContext,
    map: &str,
    items: &str,
) -> Result<String, PromptError> {
    match name {
        "date" => Ok(ctx.date.clone()),
        "session" => Ok(ctx.kind.label().to_string()),
        "commitment_map" => Ok(map.to_string()),
        "ranked_context" => Ok(items.to_string()),
        _ => Err(PromptError::UnknownVariable {
            name: name.to_string(),
            available: VARIABLES.join(", "),
        }),
    }
}

/// The template's own prose, with every placeholder emptied.
///
/// What [`super::context::TEMPLATE_OVERHEAD`] is an allowance for: the
/// instructions wrapped around the two tiers, which are charged against the
/// context window just as the tiers are.
pub fn static_text(template: &str) -> String {
    let mut out = String::new();
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);

        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => rest = &after[end + 2..],
            None => return out,
        }
    }

    out.push_str(rest);
    out
}
