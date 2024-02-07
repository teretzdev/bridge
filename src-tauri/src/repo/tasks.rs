// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Executor, Sqlite};

use crate::types::Result;

use super::Pagination;

#[derive(Serialize, Deserialize, Debug, sqlx::Type, PartialEq, Default, Clone, Copy)]
pub enum Status {
    /// Task has not been selected for execution yet.
    #[default]
    New,
    /// Task is selected for execution.
    Todo,
    /// Task is currently being executed.
    InProgress,
    /// Task is waiting for a user input.
    WaitingForUser,
    /// Task is paused by the user.
    Paused,
    /// Task is completed.
    Done,
    /// Task execution failed.
    Failed,
    /// Task canceled by the user.
    Canceled,
}

impl From<String> for Status {
    fn from(status: String) -> Self {
        match status.as_str() {
            "Todo" => Status::Todo,
            "InProgress" => Status::InProgress,
            "WaitingForUser" => Status::WaitingForUser,
            "Paused" => Status::Paused,
            "Done" => Status::Done,
            "Failed" => Status::Failed,
            "Canceled" => Status::Canceled,
            _ => Status::New,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Task {
    pub id: i64,
    pub agent_id: i64,
    /// Chat from which this task was created.
    pub origin_chat_id: Option<i64>,
    /// Chat from which this task is being controlled (between the user and the Bridge).
    pub control_chat_id: Option<i64>,
    /// Chat in which this task is being executed (between the Bridge and the agent).
    pub execution_chat_id: Option<i64>,
    pub title: String,
    pub summary: String,
    pub status: Status,
    /// Task's parent ids in a form of `1/2/3`. `None` for root tasks.
    pub ancestry: Option<String>,
    pub ancestry_level: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Task {
    #[must_use]
    pub fn parent_id(&self) -> Option<i64> {
        match self.ancestry {
            Some(ref ancestry) => ancestry
                .split('/')
                .last()
                .and_then(|id| id.parse::<i64>().ok()),
            None => None,
        }
    }
}

pub struct CreateParams<'a> {
    pub agent_id: i64,
    /// Chat from which this task was created.
    pub origin_chat_id: Option<i64>,
    pub title: &'a str,
    pub summary: &'a str,
    pub status: Status,
    /// Task's parent ids in a form of `1/2/3`. `None` for root tasks.
    pub ancestry: Option<&'a str>,
}

/// List all tasks.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list_roots<'a, E: Executor<'a, Database = Sqlite>>(
    executor: E,
    pagination: Pagination,
) -> Result<Vec<Task>> {
    if pagination.page < 1 {
        return Err(anyhow::anyhow!("`page` number must be greater than 0").into());
    }

    if pagination.per_page < 1 {
        return Err(anyhow::anyhow!("`per_page` number must be greater than 0").into());
    }

    let offset = (pagination.page - 1) * pagination.per_page;

    Ok(query_as!(
        Task,
        r#"
        SELECT
            id as "id!",
            agent_id,
            origin_chat_id,
            control_chat_id,
            execution_chat_id,
            title,
            summary,
            status,
            ancestry,
            ancestry_level,
            created_at,
            updated_at
        FROM tasks
        WHERE ancestry IS NULL
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
        pagination.per_page,
        offset,
    )
    .fetch_all(executor)
    .await
    .context("Failed to list tasks")?)
}

/// List all child tasks for given task.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list_children<'a, E: Executor<'a, Database = Sqlite>>(
    executor: E,
    id: i64,
    ancestry: Option<&'a str>,
) -> Result<Vec<Task>> {
    let current_ancestry_level: i64 = match ancestry {
        Some(ancestry) => {
            let count = ancestry.split('/').count();

            match count.try_into() {
                Ok(ancestry_level) => ancestry_level,
                Err(_) => return Err(anyhow::anyhow!("Too many ancestors").into()),
            }
        }
        None => 0,
    };

    let children_ancestry_level = current_ancestry_level
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("Maximum ancestry level reached for task with id: {id}"))?;

    let children_ancestry = if let Some(ancestry) = ancestry {
        format!("{ancestry}/{id}/%")
    } else {
        format!("{id}/%")
    };

    Ok(query_as!(
        Task,
        r#"
        SELECT
            id as "id!",
            agent_id,
            origin_chat_id,
            control_chat_id,
            execution_chat_id,
            title,
            summary,
            status,
            ancestry,
            ancestry_level,
            created_at,
            updated_at
        FROM tasks
        WHERE ancestry LIKE $1
        AND ancestry_level = $2
        ORDER BY created_at DESC
        "#,
        children_ancestry,
        children_ancestry_level,
    )
    .fetch_all(executor)
    .await
    .context("Failed to list tasks")?)
}

/// Create new task.
///
/// # Errors
///
/// Returns error if there was a problem while inserting new task.
pub async fn create<'a, E: Executor<'a, Database = Sqlite>>(
    executor: E,
    params: CreateParams<'a>,
) -> Result<Task> {
    let now = Utc::now().naive_utc();

    let ancestry_level = match params.ancestry {
        Some(ancestry) => {
            let count = ancestry.split('/').count();

            match count.try_into() {
                Ok(ancestry_level) => ancestry_level,
                Err(_) => return Err(anyhow::anyhow!("Too many ancestors").into()),
            }
        }
        None => 0,
    };

    let task = query_as!(
        Task,
        r#"
        INSERT INTO tasks (
            agent_id,
            origin_chat_id,
            title,
            summary,
            status,
            ancestry,
            ancestry_level,
            created_at,
            updated_at
        )
        VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            $8
        )
        RETURNING
            id as "id!",
            agent_id,
            origin_chat_id,
            control_chat_id,
            execution_chat_id,
            title,
            summary,
            status,
            ancestry,
            ancestry_level,
            created_at,
            updated_at
        "#,
        params.agent_id,
        params.origin_chat_id,
        params.title,
        params.summary,
        params.status,
        params.ancestry,
        ancestry_level,
        now,
    )
    .fetch_one(executor)
    .await
    .context("Failed to create task")?;

    Ok(task)
}

/// Get task by id.
///
/// # Errors
///
/// Returns error if there was a problem while fetching task.
pub async fn get<'a, E: Executor<'a, Database = Sqlite>>(executor: E, id: i64) -> Result<Task> {
    let task = query_as!(
        Task,
        r#"
        SELECT
            id as "id!",
            agent_id,
            origin_chat_id,
            control_chat_id,
            execution_chat_id,
            title,
            summary,
            status,
            ancestry,
            ancestry_level,
            created_at,
            updated_at
        FROM tasks
        WHERE id = $1
        "#,
        id,
    )
    .fetch_one(executor)
    .await
    .context("Failed to get task")?;

    Ok(task)
}

/// Delete task by id.
///
/// # Errors
///
/// Returns error if there was a problem while deleting task.
pub async fn delete<'a, E: Executor<'a, Database = Sqlite>>(executor: E, id: i64) -> Result<()> {
    query!("DELETE FROM tasks WHERE id = $1", id)
        .execute(executor)
        .await
        .context("Failed to delete task")?;

    Ok(())
}

/// Delete child tasks by parent id and ancestry.
///
/// # Errors
///
/// Returns error if there was a problem while deleting tasks.
pub async fn delete_children<'a, E: Executor<'a, Database = Sqlite>>(
    executor: E,
    id: i64,
    ancestry: Option<&'a str>,
) -> Result<()> {
    let children_ancestry = if let Some(ancestry) = ancestry {
        format!("{ancestry}/{id}/%")
    } else {
        format!("{id}/%")
    };

    query!(
        "DELETE FROM tasks WHERE ancestry LIKE $1",
        children_ancestry
    )
    .execute(executor)
    .await
    .context("Failed to delete tasks")?;

    Ok(())
}