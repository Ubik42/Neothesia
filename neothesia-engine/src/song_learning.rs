use crate::library_workspace::{Workspace, key};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub stage: String,
    #[serde(default)]
    pub goal: String,
    pub due: Option<String>,
    pub weekly_minutes: Option<u16>,
    pub target_bpm: Option<u16>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub stage: String,
    pub at: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Learning {
    #[serde(flatten)]
    pub plan: Draft,
    pub updated_at: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub transitions: Vec<Transition>,
}
impl Draft {
    pub fn validate(&self) -> Result<(), String> {
        if ![
            "planned",
            "learning",
            "polishing",
            "maintaining",
            "mastered",
            "paused",
        ]
        .contains(&self.stage.as_str())
        {
            return Err("请选择有效的学习阶段".into());
        }
        if self.goal.chars().count() > 2000 {
            return Err("学习目标最多 2000 字".into());
        }
        if self
            .due
            .as_deref()
            .is_some_and(|day| !crate::routine_commands::day_valid(day))
        {
            return Err("请选择有效的目标日期".into());
        }
        if self
            .weekly_minutes
            .is_some_and(|value| !(1..=10080).contains(&value))
        {
            return Err("每周计划时长应为 1 至 10080 分钟".into());
        }
        if self
            .target_bpm
            .is_some_and(|value| !(20..=300).contains(&value))
        {
            return Err("目标速度应为 20 至 300 BPM".into());
        }
        Ok(())
    }
}
pub fn save(
    workspace: &mut Workspace,
    path: PathBuf,
    cid: String,
    value: Option<Draft>,
    expected: Option<Learning>,
) -> Result<(), String> {
    if cid.len() != 64 || !cid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("曲目内容身份无效".into());
    }
    if workspace.learning.get(&cid) != expected.as_ref() {
        return Err("学习资料已在其他位置修改，请重新打开后再保存".into());
    }
    if let Some(value) = &value {
        value.validate()?;
    }
    if path.is_file() {
        let entry = workspace.inspect(path.clone(), true)?;
        if entry.content_id.as_deref() != Some(&cid) {
            return Err("文件内容已改变，请重新检查曲目后再设置学习目标".into());
        }
    } else if !path.exists()
        && workspace
            .entries
            .get(&key(&path))
            .is_some_and(|entry| entry.content_id.as_deref() == Some(&cid))
    {
        // Keep the known piece's learning plan editable while a source file is missing.
    } else {
        return Err("曲目文件无法核对，学习资料未保存".into());
    }
    if let Some(mut plan) = value {
        if !workspace.learning.contains_key(&cid) && workspace.learning.len() >= 10000 {
            return Err("学习曲目数量超过上限".into());
        }
        plan.goal = plan.goal.trim().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let mut transitions = expected
            .as_ref()
            .map(|old| old.transitions.clone())
            .unwrap_or_default();
        if expected
            .as_ref()
            .is_none_or(|old| old.plan.stage != plan.stage)
        {
            transitions.push(Transition {
                stage: plan.stage.clone(),
                at: now,
            });
        }
        if transitions.len() > 64 {
            transitions.drain(..transitions.len() - 64);
        }
        let title = workspace
            .entries
            .get(&key(&path))
            .and_then(|entry| entry.metadata.title.clone())
            .unwrap_or_else(|| {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });
        workspace.learning.insert(
            cid,
            Learning {
                plan,
                updated_at: now,
                title,
                transitions,
            },
        );
    } else {
        workspace.learning.remove(&cid);
    }
    Ok(())
}

impl Learning {
    pub fn validate(&self) -> Result<(), String> {
        self.plan.validate()?;
        if self.title.chars().count() > 2048 {
            return Err("学习曲目名称过长".into());
        }
        if self.transitions.len() > 64
            || self.transitions.iter().any(|entry| {
                ![
                    "planned",
                    "learning",
                    "polishing",
                    "maintaining",
                    "mastered",
                    "paused",
                ]
                .contains(&entry.stage.as_str())
            })
        {
            return Err("学习阶段记录无效".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Player,
        practice_backup::{Backup, Selection},
    };
    #[test]
    fn learning_identity_conflicts_backup_and_interrupted_workspace_restore() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!(
                "learning-test-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        std::fs::create_dir_all(&root).unwrap();
        let midi = root.join("lesson.mid");
        let alias = root.join("copy.mid");
        std::fs::write(&midi, include_bytes!("../assets/five-finger.mid")).unwrap();
        std::fs::copy(&midi, &alias).unwrap();
        let mut w = Workspace::default();
        let cid = w.inspect(midi.clone(), true).unwrap().content_id.unwrap();
        assert_eq!(
            w.inspect(alias.clone(), true)
                .unwrap()
                .content_id
                .as_deref(),
            Some(cid.as_str())
        );
        let draft = Draft {
            stage: "learning".into(),
            goal: "双手稳定演奏".into(),
            due: Some("2027-02-28".into()),
            weekly_minutes: Some(60),
            target_bpm: Some(90),
        };
        save(&mut w, midi.clone(), cid.clone(), Some(draft.clone()), None).unwrap();
        let old = w.learning[&cid].clone();
        assert!(
            save(
                &mut w,
                alias.clone(),
                cid.clone(),
                Some(draft.clone()),
                None
            )
            .is_err()
        );
        let mut invalid = draft.clone();
        invalid.due = Some("2027-02-29".into());
        assert!(invalid.validate().is_err());
        let mut changed = draft.clone();
        changed.stage = "polishing".into();
        save(&mut w, alias.clone(), cid.clone(), Some(changed), Some(old)).unwrap();
        assert_eq!(w.learning[&cid].transitions.len(), 2);
        std::fs::write(&alias, include_bytes!("../assets/basic-chords.mid")).unwrap();
        let expected = w.learning[&cid].clone();
        assert!(
            save(
                &mut w,
                alias.clone(),
                cid.clone(),
                Some(draft.clone()),
                Some(expected)
            )
            .is_err()
        );
        let other = w.inspect(alias, true).unwrap().content_id.unwrap();
        assert_ne!(other, cid);
        assert!(!w.learning.contains_key(&other));
        std::fs::remove_file(&midi).unwrap();
        let expected = w.learning[&cid].clone();
        save(
            &mut w,
            midi,
            cid.clone(),
            Some(draft.clone()),
            Some(expected),
        )
        .unwrap();
        w.save(&root).unwrap();
        assert_eq!(Workspace::load(&root).unwrap().learning, w.learning);
        let source = Player::new(
            root.clone(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2"),
            true,
        );
        let b: Backup = serde_json::from_value(
            source
                .practice_backup_snapshot(&Selection::default())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(b.version(), 3);
        let decoded = Backup::decode(&b.encode().unwrap()).unwrap();
        assert_eq!(decoded.learning, w.learning);
        let target_root = root.join("target");
        let mut target = Player::new(
            target_root.clone(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2"),
            true,
        );
        target
            .apply_practice_backup(decoded.clone(), Selection::default(), "keep".into())
            .unwrap();
        let mut local = Workspace::load(&target_root).unwrap();
        local.learning.get_mut(&cid).unwrap().plan.stage = "paused".into();
        local.save(&target_root).unwrap();
        assert_eq!(
            target.inspect_practice_backup(&decoded)["learningConflicts"],
            1
        );
        target
            .apply_practice_backup(decoded.clone(), Selection::default(), "keep".into())
            .unwrap();
        assert_eq!(
            Workspace::load(&target_root).unwrap().learning[&cid]
                .plan
                .stage,
            "paused"
        );
        target
            .apply_practice_backup(decoded, Selection::default(), "backup".into())
            .unwrap();
        assert_eq!(Workspace::load(&target_root).unwrap().learning, w.learning);
        let before = std::fs::read(target_root.join("library-workspace.json")).unwrap();
        let restore = target_root.join("practice-restore-points/restore-learning-interrupted");
        std::fs::create_dir_all(&restore).unwrap();
        for name in [
            "web-preferences.json",
            "web-practice-history.ron",
            "library-workspace.json",
        ] {
            std::fs::copy(target_root.join(name), restore.join(name)).unwrap();
        }
        std::fs::write(
            target_root.join("library-workspace.json"),
            b"partial learning write",
        )
        .unwrap();
        std::fs::write(target_root.join("practice-import-journal.json"),serde_json::to_vec(&serde_json::json!({"folder":"restore-learning-interrupted","prefs":true,"history":true,"workspace":true,"committed":false})).unwrap()).unwrap();
        crate::practice_backup::recover(&target_root).unwrap();
        assert_eq!(
            std::fs::read(target_root.join("library-workspace.json")).unwrap(),
            before
        );
    }
}
