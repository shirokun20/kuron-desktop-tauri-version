//! Application: depends on Domain only (spec §9).
//! One use case = one file, one `execute` method.

pub mod services;
pub mod usecases;

pub use usecases::{
    clear_history::ClearHistoryUseCase, clear_library::ClearLibraryUseCase,
    get_chapters::GetChaptersUseCase, get_comments::GetCommentsUseCase,
    get_content_detail::GetContentDetailUseCase,
    get_home_feed::GetHomeFeedUseCase, get_page_images::GetPageImagesUseCase,
    get_related_content::GetRelatedContentUseCase,
    list_favorites::ListFavoritesUseCase, list_history::ListHistoryUseCase,
    record_history::RecordHistoryUseCase, remove_history::RemoveHistoryUseCase,
    say_hello::SayHelloUseCase,
    search_content::SearchContentUseCase, set_favorite::SetFavoriteUseCase,
};
