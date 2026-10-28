//! `page` must cut exactly where the pre-07F algorithm cut, which re-serialized the whole
//! response after every entry. `page_v1` is a verbatim copy of that algorithm (oracle), minus the
//! per-page depth rule that B03 removed from both: tree reads now filter depth before paging.
use super::*;

fn page_v1(
    mut response: Value,
    entries: Vec<(String, Value)>,
    offset: usize,
    limit: usize,
    budget: usize,
    scope: &str,
) -> Result<Value> {
    if offset > entries.len() {
        return Err(invalid());
    }
    let mut next = offset;
    response["truncated"] = json!(false);
    response["nextCursor"] = Value::Null;
    if serde_json::to_vec(&response)?.len() > budget {
        return Err(ApiError::new("CONTEXT_ITEM_TOO_LARGE", "Context metadata exceeds the response budget; request fewer viewports or a larger budget"));
    }
    while let Some((field, value)) = entries.get(next) {
        if next - offset >= limit {
            break;
        }
        if field != "nodes"
            && response[field]
                .as_array()
                .is_some_and(|list| list.len() >= 100)
        {
            break;
        }
        if response.get(field).is_none() {
            response[field] = json!([]);
        }
        response[field].as_array_mut().unwrap().push(value.clone());
        response["nextCursor"] = json!(cursor(scope, next + 1));
        response["truncated"] = json!(true);
        if serde_json::to_vec(&response)?.len() > budget {
            response[field].as_array_mut().unwrap().pop();
            if next == offset {
                return Err(ApiError::new("CONTEXT_ITEM_TOO_LARGE", "One complete item exceeds the response budget; use a property filter or larger budget"));
            }
            break;
        }
        next += 1;
    }
    let truncated = next < entries.len();
    response["truncated"] = json!(truncated);
    response["nextCursor"] = if truncated {
        json!(cursor(scope, next))
    } else {
        Value::Null
    };
    Ok(response)
}

/// Deterministic entries of varied size: escapes, non-ASCII text, floats, nesting.
fn mixed_entries(count: usize) -> Vec<(String, Value)> {
    let texts = [
        "a",
        "Tổng quan \"đã\" xong",
        "tab\tnew\nline\\",
        "😀 emoji",
        "",
    ];
    (0..count)
        .map(|i| {
            let field = match i % 7 {
                0 | 1 => "nodes",
                2..=5 => "tokens",
                _ => "styles",
            };
            let value = json!({
                "id": format!("n{i}"),
                "text": texts[i % texts.len()].repeat(1 + i % 4),
                "x": i as f64 * 1.37,
                "props": {"width": i * 3, "nested": [i, null, true]},
            });
            (field.to_owned(), value)
        })
        .collect()
}

fn same(a: Result<Value>, b: Result<Value>, label: &str) -> Option<usize> {
    match (a, b) {
        (Ok(a), Ok(b)) => {
            assert_eq!(
                serde_json::to_string(&a).unwrap(),
                serde_json::to_string(&b).unwrap(),
                "{label}"
            );
            let next = a["nextCursor"].as_str()?;
            let decoded = URL_SAFE_NO_PAD.decode(next).unwrap();
            let cursor: Value = serde_json::from_slice(&decoded).unwrap();
            cursor["offset"].as_u64().map(|n| n as usize)
        }
        (Err(a), Err(b)) => {
            assert_eq!(a.0.code, b.0.code, "{label}");
            None
        }
        (a, b) => panic!("{label}: v1 {:?} vs new {:?}", a.is_ok(), b.is_ok()),
    }
}

#[test]
fn cumulative_page_matches_v1_over_many_budgets_and_whole_reads() {
    let entries = mixed_entries(220);
    // `tokens` starts present (key exists), `styles` is created on first use.
    let response =
        json!({"schemaVersion":1,"projectId":"p_1","nodes":[],"tokens":[],"meta":{"note":"ü"}});
    let mut budgets: Vec<usize> = (100..1_200).step_by(37).collect();
    budgets.extend((1_200..20_000).step_by(2_999));
    budgets.extend([65_536, usize::MAX]);
    let mut pages = 0;
    for budget in budgets {
        for limit in [200, 13, 1] {
            let mut offset = Some(0);
            while let Some(at) = offset {
                let label = format!("budget {budget} limit {limit} offset {at}");
                let v1 = page_v1(response.clone(), entries.clone(), at, limit, budget, "s");
                let new = page(response.clone(), entries.clone(), at, limit, budget, "s");
                offset = same(v1, new, &label);
                pages += 1;
            }
        }
    }
    assert!(pages > 500, "{pages} pages compared");
    // The 100-item cap on non-`nodes` arrays ends the first unbudgeted page.
    let first = page(response, entries, 0, 200, usize::MAX, "s").unwrap();
    assert_eq!(first["tokens"].as_array().unwrap().len(), 100);
}

#[test]
fn cumulative_page_matches_v1_where_cursor_digits_and_base64_length_change() {
    let entries: Vec<_> = (0..1_105)
        .map(|i| ("nodes".to_owned(), json!({"id":format!("n{i}")})))
        .collect();
    let response = json!({"schemaVersion":1,"nodes":[]});
    let scope = "a".repeat(64);
    for offset in (0..12)
        .chain(95..103)
        .chain(995..1_005)
        .chain([1_104, 1_105, 1_106])
    {
        for budget in (90..400).step_by(5).chain([usize::MAX]) {
            for limit in [1, 3, 200] {
                let label = format!("offset {offset} budget {budget} limit {limit}");
                let v1 = page_v1(
                    response.clone(),
                    entries.clone(),
                    offset,
                    limit,
                    budget,
                    &scope,
                );
                let new = page(
                    response.clone(),
                    entries.clone(),
                    offset,
                    limit,
                    budget,
                    &scope,
                );
                same(v1, new, &label);
            }
        }
    }
}

#[test]
fn unbudgeted_page_is_a_count_slice() {
    let items: Vec<Value> = (0..23)
        .map(|i| json!({"id":format!("n{i}"),"bounds":{"x":i,"y":i*2},"lines":[]}))
        .collect();
    let entries: Vec<_> = items
        .iter()
        .map(|v| ("nodes".to_owned(), v.clone()))
        .collect();
    let response = json!({"schemaVersion":1,"nodes":[],"status":"ready"});
    for limit in [1, 5, 22, 23, 200] {
        for offset in [0, 1, 7, 22, 23] {
            let page = page(
                response.clone(),
                entries.clone(),
                offset,
                limit,
                usize::MAX,
                "s",
            );
            let page = page.unwrap();
            let end = items.len().min(offset + limit);
            assert_eq!(page["nodes"], json!(items[offset..end]));
            assert_eq!(page["truncated"], json!(end < items.len()));
        }
    }
    let past = page(response, entries, 24, 5, usize::MAX, "s");
    assert_eq!(past.unwrap_err().0.code, "INVALID_CURSOR");
}
