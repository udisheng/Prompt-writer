use regex::Regex;
use std::sync::OnceLock;

/// Words from the rough idea that the H3-Image target sentence has to keep.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionRequirements {
    pub terms: Vec<String>,
    /// Edit prompts are checked on the sentence that starts with "The target image",
    /// before the preservation clauses. Text-to-image is checked on the whole prompt.
    pub edit: bool,
}

pub fn action_requirements(idea: &str, edit: bool) -> ActionRequirements {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut found: Vec<(usize, String)> = Vec::new();

    if idea.is_ascii() {
        let lower = idea.to_ascii_lowercase();
        let mut phrases: Vec<&str> = PHRASES.to_vec();
        phrases.sort_by_key(|phrase| std::cmp::Reverse(phrase.len()));
        for phrase in phrases {
            let mut from = 0;
            while let Some(rel) = lower[from..].find(phrase) {
                let start = from + rel;
                let end = start + phrase.len();
                if spans.iter().any(|(lo, hi)| start < *hi && end > *lo) {
                    from = end;
                    continue;
                }
                if !phrase_edges_are_boundaries(&lower, start, end) {
                    from = end;
                    continue;
                }
                spans.push((start, end));
                found.push((start, idea[start..end].to_string()));
                from = end;
            }
        }
    }

    for mat in word_regex().find_iter(idea) {
        let start = mat.start();
        let end = mat.end();
        if spans.iter().any(|(lo, hi)| start < *hi && end > *lo) {
            continue;
        }
        let word = mat.as_str();
        if !is_action_verb(word) {
            continue;
        }
        if skipped_look(idea, end) {
            continue;
        }
        spans.push((start, end));
        found.push((start, word.to_string()));
    }

    for clause in idea.split(|c| c == ',' || c == '.' || c == ';' || c == '\n' || c == '!') {
        let clause_start = clause_offset(idea, clause);
        let has_verb = word_regex().find_iter(clause).any(|mat| {
            is_action_verb(mat.as_str()) && !skipped_look(idea, clause_start + mat.end())
        });
        if !has_verb {
            continue;
        }
        for mat in word_regex().find_iter(clause) {
            let word = mat.as_str();
            if !is_action_object(word) {
                continue;
            }
            let start = clause_start + mat.start();
            let end = clause_start + mat.end();
            if spans.iter().any(|(lo, hi)| start < *hi && end > *lo) {
                continue;
            }
            spans.push((start, end));
            found.push((start, word.to_string()));
        }
    }

    found.sort_by_key(|(start, _)| *start);
    let mut terms = Vec::new();
    for (_, term) in found {
        if terms
            .iter()
            .any(|kept: &String| kept.eq_ignore_ascii_case(&term))
        {
            continue;
        }
        terms.push(term);
    }
    ActionRequirements { terms, edit }
}

pub fn action_addon(requirements: &ActionRequirements) -> String {
    if requirements.terms.is_empty() {
        return String::new();
    }
    let place = if requirements.edit {
        "the sentence that starts with \"The target image\", before any preservation sentence"
    } else {
        "the opening sentence that states the action"
    };
    let mut out = format!(
        "\n\n## Actions to keep\n\
         These words are the action in the rough idea. Put every one of them in {place}. \
         State them as body position, hand gesture, gaze, or expression. You may add concrete \
         body detail around them. Do not drop them, and do not replace them with a description \
         of the reference photo.\n"
    );
    for term in &requirements.terms {
        out.push_str("- ");
        out.push_str(term);
        out.push('\n');
    }
    out
}

pub fn missing_action_terms(output: &str, requirements: &ActionRequirements) -> Vec<String> {
    let scope = action_scope(output, requirements.edit);
    requirements
        .terms
        .iter()
        .filter(|term| !scope_contains(scope, term))
        .cloned()
        .collect()
}

pub fn action_warning(missing: &[String]) -> String {
    format!(
        "Action words missing from the rewrite: {}. They were in the idea and have to stay in the action sentence.",
        missing.join(", ")
    )
}

/// One follow-up when protected terms, action words, or both were dropped.
pub fn combined_retry(missing_terms: &[String], missing_actions: &[String]) -> Option<String> {
    if missing_terms.is_empty() && missing_actions.is_empty() {
        return None;
    }
    let mut out = String::from(
        "The rewrite dropped words that have to stay. Keep the rest of the prompt and return the same output format as before.\n",
    );
    if !missing_terms.is_empty() {
        out.push_str(
            "\nProtected terms. Put each one back exactly, same spelling, case, and punctuation, and do not wrap them in braces:\n",
        );
        for term in missing_terms {
            out.push_str("- ");
            out.push_str(term);
            out.push('\n');
        }
    }
    if !missing_actions.is_empty() {
        out.push_str(
            "\nAction words. Put each one in the action sentence (the sentence that starts with \"The target image\" on an edit, or the opening sentence on text-to-image), as body position, hand gesture, gaze, or expression:\n",
        );
        for term in missing_actions {
            out.push_str("- ");
            out.push_str(term);
            out.push('\n');
        }
    }
    Some(out)
}

fn action_scope(output: &str, edit: bool) -> &str {
    if !edit || !output.is_ascii() {
        return output;
    }
    let lower = output.to_ascii_lowercase();
    let Some(start) = lower.find("the target image") else {
        return "";
    };
    let mut end = output.len();
    for marker in [
        "is partially preserved",
        "is fully preserved",
        "do not slim",
    ] {
        if let Some(rel) = lower[start..].find(marker) {
            end = end.min(start + rel);
        }
    }
    &output[start..end]
}

fn scope_contains(scope: &str, term: &str) -> bool {
    if !scope.is_ascii() || !term.is_ascii() {
        return scope.to_lowercase().contains(&term.to_lowercase());
    }
    let scope = scope.to_ascii_lowercase();
    let term = term.to_ascii_lowercase();
    if term.contains(' ') || term.contains('-') {
        return scope.contains(&term);
    }
    scope
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|word| word == term)
}

fn phrase_edges_are_boundaries(lower: &str, start: usize, end: usize) -> bool {
    let before_ok = start == 0 || !lower.as_bytes()[start - 1].is_ascii_alphabetic();
    let after_ok = end == lower.len() || !lower.as_bytes()[end].is_ascii_alphabetic();
    before_ok && after_ok
}

fn skipped_look(idea: &str, verb_end: usize) -> bool {
    let rest = idea[verb_end..].trim_start();
    let next = rest
        .split(|c: char| !c.is_ascii_alphabetic())
        .find(|word| !word.is_empty())
        .unwrap_or("");
    next.eq_ignore_ascii_case("like") || next.eq_ignore_ascii_case("as")
}

fn is_action_verb(word: &str) -> bool {
    VERBS.iter().any(|verb| word.eq_ignore_ascii_case(verb))
}

fn is_action_object(word: &str) -> bool {
    OBJECTS
        .iter()
        .any(|object| word.eq_ignore_ascii_case(object))
}

fn clause_offset(idea: &str, clause: &str) -> usize {
    idea.find(clause).unwrap_or(0)
}

fn word_regex() -> &'static Regex {
    static WORDS: OnceLock<Regex> = OnceLock::new();
    WORDS.get_or_init(|| Regex::new(r"[A-Za-z][A-Za-z'-]*").expect("word pattern"))
}

const PHRASES: &[&str] = &[
    "faint smile",
    "slight smile",
    "small smile",
    "soft smile",
    "wide smile",
    "shy smile",
    "peace sign",
    "thumbs up",
    "thumbs-up",
    "over her shoulder",
    "over his shoulder",
    "over their shoulder",
    "over the shoulder",
    "toward the camera",
    "towards the camera",
    "toward camera",
    "towards camera",
];

const VERBS: &[&str] = &[
    "sit",
    "sits",
    "sitting",
    "seated",
    "stand",
    "stands",
    "standing",
    "walk",
    "walks",
    "walking",
    "lean",
    "leans",
    "leaning",
    "look",
    "looks",
    "looking",
    "hold",
    "holds",
    "holding",
    "held",
    "raise",
    "raises",
    "raising",
    "raised",
    "turn",
    "turns",
    "turning",
    "smile",
    "smiles",
    "smiling",
    "wave",
    "waves",
    "waving",
    "reach",
    "reaches",
    "reaching",
    "step",
    "steps",
    "stepping",
    "run",
    "runs",
    "running",
    "kneel",
    "kneels",
    "kneeling",
    "lie",
    "lies",
    "lying",
    "crouch",
    "crouches",
    "crouching",
    "bend",
    "bends",
    "bending",
    "tilt",
    "tilts",
    "tilting",
    "gaze",
    "gazes",
    "gazing",
    "glance",
    "glances",
    "glancing",
    "nod",
    "nods",
    "nodding",
    "shrug",
    "shrugs",
    "shrugging",
    "lift",
    "lifts",
    "lifting",
    "carry",
    "carries",
    "carrying",
    "sip",
    "sips",
    "sipping",
    "drink",
    "drinks",
    "drinking",
    "read",
    "reads",
    "reading",
    "point",
    "points",
    "pointing",
    "stretch",
    "stretches",
    "stretching",
    "twist",
    "twists",
    "twisting",
    "jump",
    "jumps",
    "jumping",
    "climb",
    "climbs",
    "climbing",
    "hug",
    "hugs",
    "hugging",
    "laugh",
    "laughs",
    "laughing",
    "frown",
    "frowns",
    "frowning",
    "wink",
    "winks",
    "winking",
    "stare",
    "stares",
    "staring",
    "rest",
    "rests",
    "resting",
    "recline",
    "reclines",
    "reclining",
    "stride",
    "strides",
    "striding",
    "grip",
    "grips",
    "gripping",
    "grasp",
    "grasps",
    "grasping",
    "touch",
    "touches",
    "touching",
    "tap",
    "taps",
    "tapping",
    "pour",
    "pours",
    "pouring",
    "push",
    "pushes",
    "pushing",
    "pull",
    "pulls",
    "pulling",
    "kick",
    "kicks",
    "kicking",
    "dance",
    "dances",
    "dancing",
    "spin",
    "spins",
    "spinning",
    "rotate",
    "rotates",
    "rotating",
    "facing",
];

const OBJECTS: &[&str] = &[
    "bed", "coffee", "cup", "mug", "railing", "rail", "bottle", "shoulder", "camera", "table",
    "chair", "bike", "bicycle", "phone", "stairs", "door", "window", "wheel",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_pose_gesture_and_expression_without_looks_like() {
        let idea = "A hyper-realistic middle shot of a slender lady from <Picture 1>, peace sign, wearing an outfit from <Picture 2>, background <Picture 3>, very short skirt, sitting on the bed, faint smile. 85mm f/1.8. It looks like a photo.";
        let req = action_requirements(idea, true);
        assert!(
            req.terms
                .iter()
                .any(|term| term.eq_ignore_ascii_case("sitting"))
        );
        assert!(
            req.terms
                .iter()
                .any(|term| term.eq_ignore_ascii_case("bed"))
        );
        assert!(
            req.terms
                .iter()
                .any(|term| term.eq_ignore_ascii_case("peace sign"))
        );
        assert!(
            req.terms
                .iter()
                .any(|term| term.eq_ignore_ascii_case("faint smile"))
        );
        assert!(
            !req.terms
                .iter()
                .any(|term| term.eq_ignore_ascii_case("looks"))
        );
        assert!(
            !req.terms
                .iter()
                .any(|term| term.eq_ignore_ascii_case("smile") && term != "faint smile")
        );
    }

    #[test]
    fn edit_scope_ignores_a_verb_that_appears_only_in_preservation() {
        let req = action_requirements("sitting on the bed, faint smile, peace sign", true);
        let dropped = "The woman in <Picture 1> is <Subject 1>. The target image is a portrait of <Subject 1> in the room. <Subject 1> is partially preserved; she stays as in the photo, while sitting is not described here.";
        let missing = missing_action_terms(dropped, &req);
        assert!(
            missing
                .iter()
                .any(|term| term.eq_ignore_ascii_case("sitting"))
        );
        assert!(
            missing
                .iter()
                .any(|term| term.eq_ignore_ascii_case("peace sign"))
        );
        let kept = "The target image is <Subject 1> sitting on the edge of the bed, raising both hands in a peace sign, gaze on the lens, and a faint smile. <Subject 1> is partially preserved; identity stays.";
        assert!(missing_action_terms(kept, &req).is_empty());
    }

    #[test]
    fn retry_note_names_actions_and_protected_terms() {
        let note = combined_retry(&["Mika".into()], &["walking".into()]).unwrap();
        assert!(note.contains("Mika"));
        assert!(note.contains("walking"));
        assert!(combined_retry(&[], &[]).is_none());
    }
}
