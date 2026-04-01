#![allow(dead_code)]

pub enum Step {
    OpenTransaction,
    BuildQuery,
    ExecuteQuery,
    MapRowToUser,
    CommitTransaction,
    ReturnResult,
}

pub fn user_find_trace() -> [Step; 4] {
    [
        Step::BuildQuery,
        Step::ExecuteQuery,
        Step::MapRowToUser,
        Step::ReturnResult,
    ]
}

pub fn user_create_trace() -> [Step; 5] {
    [
        Step::OpenTransaction,
        Step::BuildQuery,
        Step::ExecuteQuery,
        Step::CommitTransaction,
        Step::ReturnResult,
    ]
}

pub fn user_update_trace() -> [Step; 5] {
    [
        Step::OpenTransaction,
        Step::BuildQuery,
        Step::ExecuteQuery,
        Step::CommitTransaction,
        Step::ReturnResult,
    ]
}

pub fn user_delete_trace() -> [Step; 4] {
    [
        Step::OpenTransaction,
        Step::BuildQuery,
        Step::ExecuteQuery,
        Step::CommitTransaction,
    ]
}
