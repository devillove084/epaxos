use futures::Future;
use grpcio::{RpcStatus, UnarySink};
use std::{fmt::Error, result::Result};


/// `DatenLord` Result type
pub type EpaxosResult<T> = Result<T, Error>;

/// Send async success `gRPC` response
pub async fn async_success<R: Send>(sink: UnarySink<R>, r: R) {
    let res = sink.success(r).await;

    if let Err(e) = res {
        panic!("failed to send response, the error is: {:?}", e)
    }
}

/// Send async failure `gRPC` response
pub async fn async_fail<R>(sink: UnarySink<R>, _err: Error) {
    /*
    debug_assert_ne!(
        rsc,
        RpcStatusCode::OK,
        "the input RpcStatusCode should not be OK"
    );
    */
    //let details = format!("{}", err);
    let rs = RpcStatus::new(0);
    let res = sink.fail(rs).await;

    if let Err(e) = res {
        panic!("failed to send response, the error is: {:?}", e)
    }
}


pub fn spawn_grpc_task<R: Send + 'static>(
    sink: UnarySink<R>,
    task: impl Future<Output = EpaxosResult<R>> + Send + 'static,
) {
    smol::spawn(async move {
        let result = task.await;
        match result {
            Ok(resp) => async_success(sink, resp).await,
            Err(e) => async_fail(sink, e).await,
        }
    })
    .detach();
}