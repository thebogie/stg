use crate::venue::repository::VenueRepository;
use shared::dto::venue::VenueDto;
use shared::models::venue::Venue;
use validator::Validate;

#[async_trait::async_trait]
pub trait VenueUseCase: Send + Sync {
    async fn get_venue(&self, id: &str) -> Result<Venue, String>;
    async fn get_all_venues(&self) -> Result<Vec<Venue>, String>;
    async fn search_venues(&self, query: &str) -> Result<Vec<Venue>, String>;
    async fn search_venues_dto(&self, query: &str) -> Result<Vec<VenueDto>, String>;
    async fn search_venues_dto_with_external(&self, query: &str) -> Result<Vec<VenueDto>, String>;
    async fn get_venue_performance(&self, venue_id: &str) -> Result<serde_json::Value, String>;
    async fn get_player_venue_stats(
        &self,
        player_id: &str,
    ) -> Result<Vec<serde_json::Value>, String>;
    async fn create_venue(&self, venue_dto: VenueDto) -> Result<Venue, String>;
    async fn update_venue(&self, id: &str, venue_dto: VenueDto) -> Result<Venue, String>;
    async fn delete_venue(&self, id: &str) -> Result<(), String>;
}

pub struct VenueUseCaseImpl<R: VenueRepository> {
    pub repo: R,
}

#[async_trait::async_trait]
impl<R: VenueRepository> VenueUseCase for VenueUseCaseImpl<R> {
    async fn get_venue(&self, id: &str) -> Result<Venue, String> {
        self.repo
            .find_by_id(id)
            .await
            .ok_or_else(|| "Venue not found".to_string())
    }

    async fn get_all_venues(&self) -> Result<Vec<Venue>, String> {
        Ok(self.repo.find_all().await)
    }

    async fn search_venues(&self, query: &str) -> Result<Vec<Venue>, String> {
        Ok(self.repo.search(query).await)
    }

    async fn search_venues_dto(&self, query: &str) -> Result<Vec<VenueDto>, String> {
        Ok(self.repo.search_dto(query).await)
    }

    async fn search_venues_dto_with_external(&self, query: &str) -> Result<Vec<VenueDto>, String> {
        Ok(self.repo.search_dto_with_external(query).await)
    }

    async fn create_venue(&self, venue_dto: VenueDto) -> Result<Venue, String> {
        // Validate the DTO
        venue_dto
            .validate()
            .map_err(|e| format!("Validation error: {}", e))?;

        let venue = Venue::from(venue_dto);
        self.repo.create(venue).await
    }

    async fn update_venue(&self, id: &str, venue_dto: VenueDto) -> Result<Venue, String> {
        // Validate the DTO
        venue_dto
            .validate()
            .map_err(|e| format!("Validation error: {}", e))?;

        // Check if venue exists
        let existing_venue = self
            .repo
            .find_by_id(id)
            .await
            .ok_or_else(|| "Venue not found".to_string())?;

        // Create updated venue with existing ID and rev
        let mut updated_venue = Venue::from(venue_dto);
        updated_venue.id = existing_venue.id;
        updated_venue.rev = existing_venue.rev;

        self.repo.update(updated_venue).await
    }

    async fn delete_venue(&self, id: &str) -> Result<(), String> {
        // Check if venue exists
        self.repo
            .find_by_id(id)
            .await
            .ok_or_else(|| "Venue not found".to_string())?;

        self.repo.delete(id).await
    }

    async fn get_venue_performance(&self, venue_id: &str) -> Result<serde_json::Value, String> {
        self.repo.get_venue_performance(venue_id).await
    }

    async fn get_player_venue_stats(
        &self,
        player_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.repo.get_player_venue_stats(player_id).await
    }
}
