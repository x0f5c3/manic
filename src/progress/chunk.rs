use crate::{Connector, Error};
use hyper::header::RANGE;
use hyper::Client;
use tokio_stream::StreamExt;
use indicatif::ProgressBar;
use tracing::instrument;
use hyper::client::connect::Connect;

