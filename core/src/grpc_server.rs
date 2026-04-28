use std::net::SocketAddr;
use std::sync::Arc;

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::note::repo::Repo;
use crate::persist::SqlNoteStore;

pub mod notalking {
    pub mod v1 {
        tonic::include_proto!("notalking.v1");
    }
}

use notalking::v1::core_bridge_server::{CoreBridge, CoreBridgeServer};
use notalking::v1::{
    GetNoteContextRequest, GetNoteContextResponse, HealthCheckRequest, HealthCheckResponse,
};

pub struct CoreGrpcService {
    pub notes: Arc<SqlNoteStore>,
}

#[tonic::async_trait]
impl CoreBridge for CoreGrpcService {
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: "ok".to_owned(),
        }))
    }

    async fn get_note_context(
        &self,
        request: Request<GetNoteContextRequest>,
    ) -> Result<Response<GetNoteContextResponse>, Status> {
        let inner = request.into_inner();
        let user_id = Uuid::parse_str(&inner.user_id).map_err(|_| Status::invalid_argument("user_id"))?;
        let note_id = Uuid::parse_str(&inner.note_id).map_err(|_| Status::invalid_argument("note_id"))?;

        let note = self
            .notes
            .find_by_id(note_id)
            .await
            .map_err(|_| Status::internal("database error"))?
            .ok_or_else(|| Status::not_found("note not found"))?;

        if note.user_id != user_id {
            return Err(Status::permission_denied("note not owned by user"));
        }

        Ok(Response::new(GetNoteContextResponse {
            title: note.title,
            head_block_id: note.head_id.map(|u| u.to_string()).unwrap_or_default(),
        }))
    }
}

pub async fn serve_grpc(bind: SocketAddr, notes: Arc<SqlNoteStore>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = CoreGrpcService { notes };
    let server = CoreBridgeServer::new(service);
    tonic::transport::Server::builder()
        .add_service(server)
        .serve(bind)
        .await?;
    Ok(())
}
