use crate::*;
impl Project {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn persist(
        &self,
        conn: &mut Connection,
        before: &Snapshot,
        after: &Candidate,
        key: &str,
        request_hash: &str,
        ops: &Value,
        result: &BatchResult,
        metrics: &mut BatchMetrics,
    ) -> Result<()> {
        let time = Instant::now();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Defensive check against writers bypassing the process/root lock.
        let actual = read_meta(&tx)?.revision;
        if actual != before.revision {
            return Err(StorageError::conflict(actual));
        }
        let old = Candidate::from_snapshot(before);
        for id in old.nodes.keys().filter(|id| !after.nodes.contains_key(*id)) {
            tx.execute("DELETE FROM nodes WHERE id=?1", [id])?;
        }
        {
            let mut stmt=tx.prepare_cached("INSERT INTO nodes(id,parent_id,sort_key,kind,props_json) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET parent_id=excluded.parent_id,sort_key=excluded.sort_key,kind=excluded.kind,props_json=excluded.props_json")?;
            for n in after
                .nodes
                .values()
                .filter(|n| old.nodes.get(&n.id) != Some(*n))
            {
                stmt.execute(params![
                    n.id,
                    n.parent_id,
                    n.sort_key,
                    n.kind.as_str(),
                    serde_json::to_string(&n.props)?
                ])?;
            }
        }
        for t in after
            .tokens
            .values()
            .filter(|t| old.tokens.get(&t.id) != Some(*t))
        {
            tx.execute("INSERT INTO tokens(id,data_json) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json",params![t.id,serde_json::to_string(t)?])?;
        }
        for s in after
            .styles
            .values()
            .filter(|s| old.styles.get(&s.id) != Some(*s))
        {
            tx.execute("INSERT INTO styles(id,data_json) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json",params![s.id,serde_json::to_string(s)?])?;
        }
        for a in after
            .assets
            .values()
            .filter(|a| old.assets.get(&a.id) != Some(*a))
        {
            tx.execute("INSERT INTO assets(id,content_hash,relative_key,data_json) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json",params![a.id,a.content_hash,a.relative_key,serde_json::to_string(a)?])?;
        }
        for f in after
            .fonts
            .values()
            .filter(|f| old.fonts.get(&f.id) != Some(*f))
        {
            tx.execute("INSERT INTO fonts(id,data_json) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json",params![f.id,serde_json::to_string(f)?])?;
        }
        tx.execute("UPDATE project_meta SET current_revision=?1,updated_at=CURRENT_TIMESTAMP WHERE singleton=1",[result.committed_revision as i64])?;
        tx.execute("INSERT INTO commits(revision,base_revision,batch_id,ops_json,summary_json) VALUES(?1,?2,?3,?4,?5)",params![result.committed_revision as i64,before.revision as i64,key,serde_json::to_string(ops)?,serde_json::to_string(&result.changed_ids)?])?;
        tx.execute("INSERT INTO idempotency(key,request_hash,committed_revision,response_json) VALUES(?1,?2,?3,?4)",params![key,request_hash,result.committed_revision as i64,serde_json::to_string(result)?])?;
        metrics.sql_write_us = us(time);
        failpoint("before_commit");
        let time = Instant::now();
        tx.commit()?;
        metrics.commit_us = us(time);
        failpoint("after_commit");
        Ok(())
    }
}
