use teloxide::requests::Requester;

pub trait UiRequester:
    Requester<Err = teloxide::RequestError> + Clone + Send + Sync + 'static
{
}

impl<T> UiRequester for T where
    T: Requester<Err = teloxide::RequestError> + Clone + Send + Sync + 'static
{
}
