use crate::practice::AttemptSummary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedLadderPlan {
    pub start_percent: u16,
    pub target_percent: u16,
    pub step_percent: u16,
    pub passes_required: u8,
    pub accuracy_percent: u8,
    pub on_time_percent: Option<u8>,
    /// Zero keeps repeating; otherwise step down after this many failed rounds.
    pub failures_before_step_back: u8,
    pub attempt_limit: u16,
}
impl SpeedLadderPlan {
    pub fn validate(&self) -> Result<(), String> {
        if !(25..=200).contains(&self.start_percent)
            || !(self.start_percent..=200).contains(&self.target_percent)
            || !(1..=50).contains(&self.step_percent)
        {
            return Err("速度需在 25%～200%，目标不低于起点，步长 1～50 个百分点".into());
        }
        if !(1..=10).contains(&self.passes_required)
            || !(50..=100).contains(&self.accuracy_percent)
            || self
                .on_time_percent
                .is_some_and(|v| !(50..=100).contains(&v))
            || self.failures_before_step_back > 10
            || !(1..=500).contains(&self.attempt_limit)
        {
            return Err(
                "达标轮数需 1～10，正确率/准时率需 50%～100%，本次最多练习 1～500 轮".into(),
            );
        }
        Ok(())
    }
    pub fn speeds(&self) -> Vec<u16> {
        let mut speeds = vec![self.start_percent];
        while *speeds.last().unwrap() < self.target_percent {
            speeds.push((*speeds.last().unwrap() + self.step_percent).min(self.target_percent));
        }
        speeds
    }
}
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedLadderProgress {
    pub stage: usize,
    pub success_streak: u8,
    pub failure_streak: u8,
    pub total_rounds: u32,
    pub batch_rounds: u16,
    pub completed: bool,
    pub limited: bool,
    pub last_result: String,
    pub last_accuracy: Option<f32>,
    pub last_on_time: Option<f32>,
}
impl SpeedLadderProgress {
    pub fn speed(&self, plan: &SpeedLadderPlan) -> u16 {
        plan.speeds()[self.stage.min(plan.speeds().len() - 1)]
    }
    pub fn resume(&mut self) {
        self.limited = false;
        self.batch_rounds = 0;
        self.last_result = "继续当前速度".into();
    }
    pub fn evaluate(
        &mut self,
        plan: &SpeedLadderPlan,
        summary: &AttemptSummary,
        target_notes: usize,
    ) {
        if self.completed || self.limited {
            return;
        }
        let score = summary.overall;
        let accuracy = score.accuracy();
        let on_time = (score.matched_notes > 0)
            .then(|| score.on_time_notes as f32 / score.matched_notes as f32);
        let complete = target_notes > 0 && score.matched_notes + score.missed_notes >= target_notes;
        let passed = complete
            && accuracy.is_some_and(|v| v + 0.00001 >= f32::from(plan.accuracy_percent) / 100.)
            && plan.on_time_percent.is_none_or(|threshold| {
                on_time.is_some_and(|v| v + 0.00001 >= f32::from(threshold) / 100.)
            });
        self.total_rounds += 1;
        self.batch_rounds += 1;
        self.last_accuracy = accuracy;
        self.last_on_time = on_time;
        if passed {
            self.success_streak += 1;
            self.failure_streak = 0;
            self.last_result = "本轮达标".into();
            if self.success_streak >= plan.passes_required {
                if self.stage + 1 == plan.speeds().len() {
                    self.completed = true;
                    self.last_result = "目标速度已达标".into();
                } else {
                    self.stage += 1;
                    self.success_streak = 0;
                    self.last_result = "已晋级下一速度".into();
                }
            }
        } else {
            self.success_streak = 0;
            self.failure_streak = self.failure_streak.saturating_add(1);
            self.last_result = if !complete {
                "目标音符未练完"
            } else if !accuracy
                .is_some_and(|v| v + 0.00001 >= f32::from(plan.accuracy_percent) / 100.)
            {
                "正确率未达标"
            } else {
                "准时率未达标"
            }
            .into();
            if plan.failures_before_step_back > 0
                && self.failure_streak >= plan.failures_before_step_back
            {
                if self.stage > 0 {
                    self.stage -= 1;
                    self.last_result = "连续未达标，回退一级".into();
                }
                self.failure_streak = 0;
            }
        }
        if !self.completed && self.batch_rounds >= plan.attempt_limit {
            self.limited = true;
            self.last_result = "本次轮数已到上限".into();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consecutive_passes_final_stage_backoff_limit_and_partial_evidence() {
        let plan = SpeedLadderPlan {
            start_percent: 50,
            target_percent: 65,
            step_percent: 10,
            passes_required: 2,
            accuracy_percent: 90,
            on_time_percent: Some(80),
            failures_before_step_back: 2,
            attempt_limit: 20,
        };
        assert_eq!(plan.speeds(), vec![50, 60, 65]);
        let mut good = AttemptSummary::default();
        good.overall.matched_notes = 10;
        good.overall.on_time_notes = 10;
        let mut p = SpeedLadderProgress::default();
        p.evaluate(&plan, &good, 20);
        assert_eq!(p.success_streak, 0);
        p.evaluate(&plan, &good, 10);
        p.evaluate(&plan, &good, 10);
        assert_eq!(p.speed(&plan), 60);
        let mut bad = good.clone();
        bad.overall.on_time_notes = 0;
        p.evaluate(&plan, &bad, 10);
        p.evaluate(&plan, &bad, 10);
        assert_eq!(p.speed(&plan), 50);
        for _ in 0..6 {
            p.evaluate(&plan, &good, 10);
        }
        assert!(p.completed);
        assert_eq!(p.speed(&plan), 65);
        let limited = SpeedLadderPlan {
            attempt_limit: 1,
            ..plan
        };
        let mut p = SpeedLadderProgress::default();
        p.evaluate(&limited, &bad, 10);
        assert!(p.limited);
        p.resume();
        assert!(!p.limited);
        assert_eq!(p.batch_rounds, 0);
        assert_eq!(p.total_rounds, 1);
    }
}
