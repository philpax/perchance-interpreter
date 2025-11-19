/// Grammar transformation functions for text manipulation
pub fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn to_sentence_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn to_plural(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    let lower = s_trimmed.to_lowercase();

    // Common irregular plurals
    let irregulars = [
        ("child", "children"),
        ("person", "people"),
        ("man", "men"),
        ("woman", "women"),
        ("tooth", "teeth"),
        ("foot", "feet"),
        ("mouse", "mice"),
        ("goose", "geese"),
        ("ox", "oxen"),
        ("sheep", "sheep"),
        ("deer", "deer"),
        ("fish", "fish"),
    ];

    for (singular, plural) in &irregulars {
        if lower == *singular {
            return plural.to_string();
        }
    }

    // Regular plural rules
    if lower.ends_with("s")
        || lower.ends_with("ss")
        || lower.ends_with("sh")
        || lower.ends_with("ch")
        || lower.ends_with("x")
        || lower.ends_with("z")
    {
        return format!("{}es", s_trimmed);
    } else if lower.ends_with("y") && s_trimmed.len() > 1 {
        let second_last = s_trimmed.chars().nth(s_trimmed.len() - 2).unwrap();
        if !"aeiou".contains(second_last.to_ascii_lowercase()) {
            return format!("{}ies", &s_trimmed[..s_trimmed.len() - 1]);
        }
    } else if lower.ends_with("f") {
        return format!("{}ves", &s_trimmed[..s_trimmed.len() - 1]);
    } else if lower.ends_with("fe") {
        return format!("{}ves", &s_trimmed[..s_trimmed.len() - 2]);
    } else if lower.ends_with("o") && s_trimmed.len() > 1 {
        let second_last = s_trimmed.chars().nth(s_trimmed.len() - 2).unwrap();
        if !"aeiou".contains(second_last.to_ascii_lowercase()) {
            return format!("{}es", s_trimmed);
        }
    }

    // Default: add 's'
    format!("{}s", s_trimmed)
}

pub fn to_past_tense(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    let lower = s_trimmed.to_lowercase();

    // Common irregular verbs
    let irregulars = [
        ("be", "was"),
        ("have", "had"),
        ("do", "did"),
        ("say", "said"),
        ("go", "went"),
        ("get", "got"),
        ("make", "made"),
        ("know", "knew"),
        ("think", "thought"),
        ("take", "took"),
        ("see", "saw"),
        ("come", "came"),
        ("want", "wanted"),
        ("give", "gave"),
        ("use", "used"),
        ("find", "found"),
        ("tell", "told"),
        ("ask", "asked"),
        ("work", "worked"),
        ("feel", "felt"),
        ("leave", "left"),
        ("put", "put"),
        ("mean", "meant"),
        ("keep", "kept"),
        ("let", "let"),
        ("begin", "began"),
        ("seem", "seemed"),
        ("help", "helped"),
        ("show", "showed"),
        ("hear", "heard"),
        ("play", "played"),
        ("run", "ran"),
        ("move", "moved"),
        ("live", "lived"),
        ("believe", "believed"),
        ("bring", "brought"),
        ("write", "wrote"),
        ("sit", "sat"),
        ("stand", "stood"),
        ("lose", "lost"),
        ("pay", "paid"),
        ("meet", "met"),
        ("include", "included"),
        ("continue", "continued"),
        ("set", "set"),
        ("learn", "learned"),
        ("change", "changed"),
        ("lead", "led"),
        ("understand", "understood"),
        ("watch", "watched"),
        ("follow", "followed"),
        ("stop", "stopped"),
        ("create", "created"),
        ("speak", "spoke"),
        ("read", "read"),
        ("spend", "spent"),
        ("grow", "grew"),
        ("open", "opened"),
        ("walk", "walked"),
        ("win", "won"),
        ("teach", "taught"),
        ("offer", "offered"),
        ("remember", "remembered"),
        ("consider", "considered"),
        ("appear", "appeared"),
        ("buy", "bought"),
        ("serve", "served"),
        ("die", "died"),
        ("send", "sent"),
        ("build", "built"),
        ("stay", "stayed"),
        ("fall", "fell"),
        ("cut", "cut"),
        ("reach", "reached"),
        ("kill", "killed"),
        ("raise", "raised"),
        ("pass", "passed"),
        ("sell", "sold"),
        ("decide", "decided"),
        ("return", "returned"),
        ("explain", "explained"),
        ("hope", "hoped"),
        ("develop", "developed"),
        ("carry", "carried"),
        ("break", "broke"),
        ("receive", "received"),
        ("agree", "agreed"),
        ("support", "supported"),
        ("hit", "hit"),
        ("produce", "produced"),
        ("eat", "ate"),
        ("cover", "covered"),
        ("catch", "caught"),
        ("draw", "drew"),
    ];

    for (present, past) in &irregulars {
        if lower == *present {
            return past.to_string();
        }
    }

    // Regular past tense rules
    if lower.ends_with("e") {
        return format!("{}d", s_trimmed);
    } else if lower.ends_with("y") && s_trimmed.len() > 1 {
        let second_last = s_trimmed.chars().nth(s_trimmed.len() - 2).unwrap();
        if !"aeiou".contains(second_last.to_ascii_lowercase()) {
            return format!("{}ied", &s_trimmed[..s_trimmed.len() - 1]);
        }
    }

    // Default: add 'ed'
    format!("{}ed", s_trimmed)
}

pub fn to_possessive(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    // If it ends with 's', just add apostrophe
    // Otherwise add apostrophe + s
    if s_trimmed.ends_with('s') {
        format!("{}'", s_trimmed)
    } else {
        format!("{}'s", s_trimmed)
    }
}

pub fn to_future_tense(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    // Future tense in English is typically "will" + base form
    format!("will {}", s_trimmed)
}

pub fn to_present_tense(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    let lower = s_trimmed.to_lowercase();

    // Common irregular present tense (third person singular)
    let irregulars = [
        ("be", "is"),
        ("have", "has"),
        ("do", "does"),
        ("go", "goes"),
        ("was", "is"),
        ("were", "are"),
        ("had", "has"),
        ("did", "does"),
        ("went", "goes"),
        ("got", "gets"),
        ("made", "makes"),
        ("knew", "knows"),
        ("thought", "thinks"),
        ("took", "takes"),
        ("saw", "sees"),
        ("came", "comes"),
        ("gave", "gives"),
        ("found", "finds"),
        ("told", "tells"),
        ("asked", "asks"),
        ("felt", "feels"),
        ("left", "leaves"),
        ("put", "puts"),
        ("meant", "means"),
        ("kept", "keeps"),
        ("let", "lets"),
        ("began", "begins"),
        ("seemed", "seems"),
        ("showed", "shows"),
        ("heard", "hears"),
        ("ran", "runs"),
        ("moved", "moves"),
        ("lived", "lives"),
        ("brought", "brings"),
        ("wrote", "writes"),
        ("sat", "sits"),
        ("stood", "stands"),
        ("lost", "loses"),
        ("paid", "pays"),
        ("met", "meets"),
        ("set", "sets"),
        ("led", "leads"),
        ("understood", "understands"),
        ("followed", "follows"),
        ("stopped", "stops"),
        ("spoke", "speaks"),
        ("read", "reads"),
        ("spent", "spends"),
        ("grew", "grows"),
        ("walked", "walks"),
        ("won", "wins"),
        ("taught", "teaches"),
        ("remembered", "remembers"),
        ("appeared", "appears"),
        ("bought", "buys"),
        ("served", "serves"),
        ("died", "dies"),
        ("sent", "sends"),
        ("built", "builds"),
        ("stayed", "stays"),
        ("fell", "falls"),
        ("cut", "cuts"),
        ("reached", "reaches"),
        ("killed", "kills"),
        ("raised", "raises"),
        ("passed", "passes"),
        ("sold", "sells"),
        ("decided", "decides"),
        ("returned", "returns"),
        ("explained", "explains"),
        ("hoped", "hopes"),
        ("carried", "carries"),
        ("broke", "breaks"),
        ("received", "receives"),
        ("agreed", "agrees"),
        ("hit", "hits"),
        ("produced", "produces"),
        ("ate", "eats"),
        ("caught", "catches"),
        ("drew", "draws"),
    ];

    for (past, present) in &irregulars {
        if lower == *past {
            return present.to_string();
        }
    }

    // If it already looks like present tense (ends with common patterns)
    if lower.ends_with("s") || lower.ends_with("es") {
        return s_trimmed.to_string();
    }

    // Regular present tense (third person singular)
    if lower.ends_with("y") && s_trimmed.len() > 1 {
        let second_last = s_trimmed.chars().nth(s_trimmed.len() - 2).unwrap();
        if !"aeiou".contains(second_last.to_ascii_lowercase()) {
            return format!("{}ies", &s_trimmed[..s_trimmed.len() - 1]);
        }
    } else if lower.ends_with("s")
        || lower.ends_with("ss")
        || lower.ends_with("sh")
        || lower.ends_with("ch")
        || lower.ends_with("x")
        || lower.ends_with("z")
        || lower.ends_with("o")
    {
        return format!("{}es", s_trimmed);
    }

    // Default: add 's'
    format!("{}s", s_trimmed)
}

pub fn to_negative_form(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    let lower = s_trimmed.to_lowercase();

    // Special cases for common verbs - all add "not" after the verb
    if lower == "is"
        || lower == "are"
        || lower == "am"
        || lower == "was"
        || lower == "were"
        || lower == "have"
        || lower == "has"
        || lower == "had"
        || lower == "do"
        || lower == "does"
        || lower == "did"
        || lower == "will"
        || lower == "would"
        || lower == "should"
        || lower == "could"
        || lower == "can"
        || lower == "may"
        || lower == "might"
        || lower == "must"
    {
        return format!("{} not", s_trimmed);
    }

    // For regular verbs, use "does not" + base form
    // This is a simplification; ideally we'd convert to base form
    format!("does not {}", s_trimmed)
}

pub fn to_singular(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    let lower = s_trimmed.to_lowercase();

    // Common irregular plurals (reversed from to_plural)
    let irregulars = [
        ("children", "child"),
        ("people", "person"),
        ("men", "man"),
        ("women", "woman"),
        ("teeth", "tooth"),
        ("feet", "foot"),
        ("mice", "mouse"),
        ("geese", "goose"),
        ("oxen", "ox"),
        ("sheep", "sheep"),
        ("deer", "deer"),
        ("fish", "fish"),
    ];

    for (plural, singular) in &irregulars {
        if lower == *plural {
            return singular.to_string();
        }
    }

    // Regular plural rules (reversed)
    if lower.ends_with("ies") && s_trimmed.len() > 3 {
        return format!("{}y", &s_trimmed[..s_trimmed.len() - 3]);
    } else if lower.ends_with("ves") && s_trimmed.len() > 3 {
        // Could be knife -> knives or life -> lives
        return format!("{}fe", &s_trimmed[..s_trimmed.len() - 3]);
    } else if lower.ends_with("oes") && s_trimmed.len() > 3 {
        return format!("{}o", &s_trimmed[..s_trimmed.len() - 2]);
    } else if lower.ends_with("ses") && s_trimmed.len() > 3 {
        return s_trimmed[..s_trimmed.len() - 2].to_string();
    } else if lower.ends_with("xes")
        || lower.ends_with("zes")
        || lower.ends_with("ches")
        || lower.ends_with("shes")
    {
        if s_trimmed.len() > 2 {
            return s_trimmed[..s_trimmed.len() - 2].to_string();
        }
    } else if lower.ends_with("s") && !lower.ends_with("ss") {
        // Simple plural - just remove 's'
        if s_trimmed.len() > 1 {
            return s_trimmed[..s_trimmed.len() - 1].to_string();
        }
    }

    // If no rule matched, return as-is
    s_trimmed.to_string()
}
