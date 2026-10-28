use super::values::*;
use crate::{error::*, model::*};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone)]
pub(crate) struct Candidate {
    pub(super) theme: String,
    pub nodes: BTreeMap<String, Node>,
    pub tokens: BTreeMap<String, Token>,
    pub styles: BTreeMap<String, Style>,
    pub assets: BTreeMap<String, Asset>,
    pub fonts: BTreeMap<String, Font>,
    pub notes: BTreeMap<String, Note>,
    /// Node IDs deleted by this batch; links to them are pruned before validation.
    pub(super) deleted: BTreeSet<String>,
}
impl Candidate {
    pub fn from_snapshot(s: &Snapshot) -> Self {
        Self {
            theme: "default".into(),
            nodes: s.nodes.iter().cloned().map(|x| (x.id.clone(), x)).collect(),
            tokens: s
                .tokens
                .iter()
                .cloned()
                .map(|x| (x.id.clone(), x))
                .collect(),
            styles: s
                .styles
                .iter()
                .cloned()
                .map(|x| (x.id.clone(), x))
                .collect(),
            assets: s
                .assets
                .iter()
                .cloned()
                .map(|x| (x.id.clone(), x))
                .collect(),
            fonts: s.fonts.iter().cloned().map(|x| (x.id.clone(), x)).collect(),
            notes: s.notes.iter().cloned().map(|x| (x.id.clone(), x)).collect(),
            deleted: BTreeSet::new(),
        }
    }
}
impl Candidate {
    pub fn validate(&self) -> Result<()> {
        if self.nodes.len() > MAX_NODES {
            return Err(StorageError::validation(
                "nodes",
                "Project exceeds 20000 authored nodes",
            ));
        }
        self.validate_notes()?;
        self.validate_navigation()?;
        let themes = self.validate_tokens()?;
        for theme in themes {
            let mut themed = self.clone();
            themed.theme = theme;
            themed.validate_content()?;
        }
        Ok(())
    }
    fn validate_content(&self) -> Result<()> {
        for style in self.styles.values() {
            self.style(style)?;
        }
        for node in self.nodes.values() {
            self.node(node)?;
        }
        Ok(())
    }
    fn place(
        &mut self,
        id: &str,
        parent: Option<String>,
        index: Option<usize>,
        changed: &mut BTreeSet<String>,
    ) -> Result<()> {
        let mut siblings: Vec<_> = self
            .nodes
            .values()
            .filter(|n| n.id != id && n.parent_id == parent)
            .map(|n| (n.sort_key, n.id.clone()))
            .collect();
        siblings.sort();
        let i = index.unwrap_or(siblings.len());
        if i > siblings.len() {
            return Err(StorageError::validation(
                "index",
                "Sibling index is out of bounds",
            ));
        }
        siblings.insert(i, (0, id.to_string()));
        for (i, (_, key)) in siblings.into_iter().enumerate() {
            let n = self.nodes.get_mut(&key).unwrap();
            if n.sort_key != i as i64 || n.parent_id != parent {
                n.sort_key = i as i64;
                n.parent_id = parent.clone();
                changed.insert(key);
            }
        }
        Ok(())
    }
    pub fn apply(&mut self, op: &Operation, changed: &mut BTreeSet<String>) -> Result<()> {
        match op {
            Operation::CreateNode {
                id: key,
                kind,
                parent_id,
                props,
                index,
            } => {
                id(key)?;
                if self.nodes.contains_key(key) {
                    return Err(StorageError::validation("id", "Node ID already exists"));
                }
                if self.notes.contains_key(key) {
                    return Err(StorageError::validation(
                        "id",
                        "ID already belongs to a note",
                    ));
                }
                self.nodes.insert(
                    key.clone(),
                    Node {
                        id: key.clone(),
                        kind: kind.clone(),
                        parent_id: parent_id.clone(),
                        props: props.clone(),
                        sort_key: 0,
                    },
                );
                self.node(&self.nodes[key])?;
                self.place(key, parent_id.clone(), *index, changed)?;
                changed.insert(key.clone());
            }
            Operation::UpdateNode { id: key, props } => {
                let update = props.as_object().ok_or_else(|| {
                    StorageError::validation("props", "Update properties must be an object")
                })?;
                let n = self
                    .nodes
                    .get_mut(key)
                    .ok_or_else(|| StorageError::validation("id", "Node does not exist"))?;
                n.props.as_object_mut().unwrap().extend(update.clone());
                self.node(&self.nodes[key])?;
                changed.insert(key.clone());
            }
            Operation::ReparentNode {
                id: key,
                parent_id,
                index,
            } => {
                let n = self
                    .nodes
                    .get_mut(key)
                    .ok_or_else(|| StorageError::validation("id", "Node does not exist"))?;
                n.parent_id = Some(parent_id.clone());
                self.node(&self.nodes[key])?;
                self.place(key, Some(parent_id.clone()), *index, changed)?;
                changed.insert(key.clone());
            }
            Operation::ReorderNode { id: key, index } => {
                let parent = self
                    .nodes
                    .get(key)
                    .ok_or_else(|| StorageError::validation("id", "Node does not exist"))?
                    .parent_id
                    .clone();
                self.place(key, parent, Some(*index), changed)?;
                changed.insert(key.clone());
            }
            Operation::DeleteSubtree { id: key } => {
                if !self.nodes.contains_key(key) {
                    return Err(StorageError::validation("id", "Node does not exist"));
                }
                let mut remove = BTreeSet::from([key.clone()]);
                loop {
                    let children: Vec<_> = self
                        .nodes
                        .values()
                        .filter(|n| {
                            !remove.contains(&n.id)
                                && n.parent_id.as_ref().is_some_and(|p| remove.contains(p))
                        })
                        .map(|n| n.id.clone())
                        .collect();
                    if children.is_empty() {
                        break;
                    }
                    remove.extend(children);
                }
                self.cascade_notes(&remove, changed);
                self.deleted.extend(remove.iter().cloned());
                for key in remove {
                    self.nodes.remove(&key);
                    changed.insert(key);
                }
            }
            Operation::UpsertToken { token } => {
                id(&token.id)?;
                self.tokens.insert(token.id.clone(), token.clone());
                self.validate_tokens()?;
                changed.insert(token.id.clone());
            }
            Operation::UpsertStyle { style } => {
                self.style(style)?;
                self.styles.insert(style.id.clone(), style.clone());
                changed.insert(style.id.clone());
            }
            Operation::CreateNote { .. }
            | Operation::ReplyNote { .. }
            | Operation::UpdateNote { .. }
            | Operation::DeleteNote { .. } => self.apply_note(op, changed)?,
        }
        Ok(())
    }
}
