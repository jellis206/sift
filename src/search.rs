/// Case-insensitive literal matching: any (or, with `all`, every) term must
/// appear, and no excluded term may.
pub struct Query {
    terms: Vec<String>,
    exclude: Vec<String>,
    all: bool,
}

impl Query {
    pub fn new(terms: &[String], exclude: &[String], all: bool) -> Self {
        let lower = |v: &[String]| v.iter().map(|t| t.to_lowercase()).collect();
        Query {
            terms: lower(terms),
            exclude: lower(exclude),
            all,
        }
    }

    pub fn matches(&self, text: &str) -> bool {
        // ponytail: lowercases every prompt and scans once per term; switch to
        // aho-corasick if profiling shows matching dominates over JSON parsing.
        let text = text.to_lowercase();
        let has = |t: &String| text.contains(t.as_str());
        if self.exclude.iter().any(has) {
            return false;
        }
        if self.all {
            self.terms.iter().all(has)
        } else {
            self.terms.iter().any(has)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Query;

    fn q(terms: &[&str], exclude: &[&str], all: bool) -> Query {
        let s = |v: &[&str]| v.iter().map(|t| t.to_string()).collect::<Vec<_>>();
        Query::new(&s(terms), &s(exclude), all)
    }

    #[test]
    fn or_needs_any_term() {
        let query = q(&["review", "deploy"], &[], false);
        assert!(query.matches("please Review this"));
        assert!(!query.matches("nothing here"));
    }

    #[test]
    fn and_needs_every_term_in_any_order() {
        let query = q(&["pr", "review"], &[], true);
        assert!(query.matches("review the PR"));
        assert!(!query.matches("review the diff"));
    }

    #[test]
    fn not_rejects_and_phrases_work() {
        let query = q(&["review"], &["already merged"], false);
        assert!(query.matches("review it, merged later"));
        assert!(!query.matches("review it, ALREADY MERGED"));
    }

    #[test]
    fn unicode_case_insensitive() {
        assert!(q(&["ÉCOLE"], &[], false).matches("à l'école"));
    }
}
