use regex::{Regex, RegexBuilder};
use serde::Serialize;

use crate::domain::classifier_blocks::{ClassifierPattern, ClassifierRules};

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ClassifierRulesValidationIssue {
    pub path: String,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ClassifierPatternValidation {
    pub path: String,
    pub capture_count: usize,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ClassifierRulesValidation {
    pub valid: bool,
    pub issues: Vec<ClassifierRulesValidationIssue>,
    pub patterns: Vec<ClassifierPatternValidation>,
}

pub fn compile_classifier_pattern(text: &str) -> Result<Regex, regex::Error> {
    RegexBuilder::new(text)
        .case_insensitive(true)
        .multi_line(true)
        .build()
}

pub fn validate_classifier_rules(rules: &ClassifierRules) -> ClassifierRulesValidation {
    let mut issues = Vec::new();
    let mut patterns = Vec::new();

    for (index, pattern) in rules.match_patterns.iter().enumerate() {
        validate_pattern(
            pattern,
            &format!("match_patterns[{index}]"),
            &mut issues,
            &mut patterns,
        );
    }

    for (index, rule) in rules.child_rules.iter().enumerate() {
        validate_pattern(
            &rule.pattern,
            &format!("child_rules[{index}].pattern"),
            &mut issues,
            &mut patterns,
        );
    }

    ClassifierRulesValidation {
        valid: issues.is_empty(),
        issues,
        patterns,
    }
}

fn validate_pattern(
    pattern: &ClassifierPattern,
    path: &str,
    issues: &mut Vec<ClassifierRulesValidationIssue>,
    patterns: &mut Vec<ClassifierPatternValidation>,
) {
    let text = pattern
        .text
        .as_deref()
        .filter(|text| !text.trim().is_empty());
    let has_metadata = pattern
        .metadata
        .as_ref()
        .is_some_and(|metadata| !metadata.is_empty());

    if text.is_none() && !has_metadata {
        issues.push(ClassifierRulesValidationIssue {
            path: path.to_string(),
            code: "empty_pattern",
            message: "A pattern must contain text or at least one metadata condition".to_string(),
        });
        return;
    }

    let Some(text) = text else {
        return;
    };

    match compile_classifier_pattern(text) {
        Ok(regex) => patterns.push(ClassifierPatternValidation {
            path: format!("{path}.text"),
            capture_count: regex.captures_len().saturating_sub(1),
        }),
        Err(error) => issues.push(ClassifierRulesValidationIssue {
            path: format!("{path}.text"),
            code: "invalid_regex",
            message: format!("Invalid regular expression: {error}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::domain::classifier_blocks::{
        ClassifierChildRule, ClassifierPattern, ClassifierRules,
    };

    use super::validate_classifier_rules;

    fn rules(patterns: Vec<ClassifierPattern>) -> ClassifierRules {
        ClassifierRules {
            continue_after_match: false,
            match_patterns: patterns,
            match_actions: HashMap::new(),
            child_rules: Vec::new(),
        }
    }

    #[test]
    fn requires_at_least_one_match_pattern() {
        let result = validate_classifier_rules(&rules(Vec::new()));

        assert!(!result.valid);
        assert_eq!(result.issues[0].path, "match_patterns");
        assert_eq!(result.issues[0].code, "required");
    }

    #[test]
    fn rejects_empty_parent_and_child_patterns() {
        let mut input = rules(vec![ClassifierPattern {
            text: Some("  ".to_string()),
            metadata: None,
        }]);
        input.child_rules.push(ClassifierChildRule {
            pattern: ClassifierPattern {
                text: None,
                metadata: Some(HashMap::new()),
            },
            modifiers: None,
            actions: HashMap::new(),
        });

        let result = validate_classifier_rules(&input);

        assert!(!result.valid);
        assert_eq!(result.issues.len(), 2);
        assert_eq!(result.issues[0].path, "match_patterns[0]");
        assert_eq!(result.issues[1].path, "child_rules[0].pattern");
    }

    #[test]
    fn accepts_metadata_only_patterns() {
        let result = validate_classifier_rules(&rules(vec![ClassifierPattern {
            text: None,
            metadata: Some(HashMap::from([("status".to_string(), "ready".to_string())])),
        }]));

        assert!(result.valid);
        assert!(result.patterns.is_empty());
    }

    #[test]
    fn reports_regex_errors_and_capture_counts() {
        let mut input = rules(vec![ClassifierPattern {
            text: Some("Invoice ([0-9]+)".to_string()),
            metadata: None,
        }]);
        input.child_rules.push(ClassifierChildRule {
            pattern: ClassifierPattern {
                text: Some("(?P<month>\\d{2})/(\\d{4})".to_string()),
                metadata: None,
            },
            modifiers: None,
            actions: HashMap::new(),
        });

        let result = validate_classifier_rules(&input);

        assert!(result.valid);
        assert_eq!(result.patterns[0].capture_count, 1);
        assert_eq!(result.patterns[1].capture_count, 2);

        input.match_patterns[0].text = Some("(".to_string());
        let invalid = validate_classifier_rules(&input);
        assert!(!invalid.valid);
        assert_eq!(invalid.issues[0].path, "match_patterns[0].text");
        assert_eq!(invalid.issues[0].code, "invalid_regex");
    }
}
