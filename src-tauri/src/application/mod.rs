//! Application: depends on Domain only (spec §9).
//! One use case = one file, one `execute` method.

pub mod services;
pub mod usecases;

pub use usecases::{
    get_chapters::GetChaptersUseCase, get_content_detail::GetContentDetailUseCase,
    get_home_feed::GetHomeFeedUseCase, get_page_images::GetPageImagesUseCase,
    say_hello::SayHelloUseCase, search_content::SearchContentUseCase,
};
