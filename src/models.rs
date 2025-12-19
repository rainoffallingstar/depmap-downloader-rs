use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub id: String,
    pub name: String,
    pub release_date: Option<DateTime<Utc>>,
    pub files: Vec<DownloadFile>,
    pub is_current: bool,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFile {
    pub id: Option<i64>,
    pub filename: String,
    pub url: String,
    pub md5_hash: Option<String>,
    pub size: Option<u64>,
    pub data_type: Option<String>,
    pub release_id: Option<String>,
    pub is_downloaded: bool,
    pub download_path: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub id: String,
    pub display_name: String,
    pub data_type: String,
    pub download_entry_url: Option<String>,
    pub associated_files: Vec<DownloadFile>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellLine {
    pub id: String,
    pub name: String,
    pub lineage: Option<String>,
    pub tissue: Option<String>,
    pub datasets_available: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneDependency {
    pub id: Option<i64>,
    pub entrez_id: u32,
    pub gene: String,
    pub dataset: String,
    pub dependent_cell_lines: f64,
    pub cell_lines_with_data: f64,
    pub strongly_selective: bool,
    pub common_essential: bool,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub dataset_count: usize,
    pub file_count: usize,
    pub release_count: usize,
    pub cell_line_count: usize,
    pub gene_dependency_count: usize,
    pub total_size_mb: u64,
    pub last_updated: Option<DateTime<Utc>>,
}

// API Response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetApiResponse {
    pub id: String,
    pub display_name: String,
    pub data_type: String,
    pub download_entry_url: Option<String>,
}

// For CSV parsing of downloads.csv
#[derive(Debug, Deserialize)]
pub struct DownloadCsvRow {
    pub release: String,
    pub release_date: String,
    pub filename: String,
    pub url: String,
    pub md5_hash: String,
}

// For CSV parsing of gene dependencies
#[derive(Debug, Deserialize)]
pub struct GeneDependencyCsvRow {
    #[serde(rename = "Entrez Id")]
    pub entrez_id: u32,
    pub gene: String,
    pub dataset: String,
    #[serde(rename = "Dependent Cell Lines")]
    pub dependent_cell_lines: f64,
    #[serde(rename = "Cell Lines with Data")]
    pub cell_lines_with_data: f64,
    #[serde(rename = "Strongly Selective")]
    pub strongly_selective: String, // "True"/"False" strings
    #[serde(rename = "Common Essential")]
    pub common_essential: String,   // "True"/"False" strings
}

impl From<GeneDependencyCsvRow> for GeneDependency {
    fn from(row: GeneDependencyCsvRow) -> Self {
        GeneDependency {
            id: None,
            entrez_id: row.entrez_id,
            gene: row.gene,
            dataset: row.dataset,
            dependent_cell_lines: row.dependent_cell_lines,
            cell_lines_with_data: row.cell_lines_with_data,
            strongly_selective: row.strongly_selective.parse().unwrap_or(false),
            common_essential: row.common_essential.parse().unwrap_or(false),
            created_at: Some(Utc::now()),
        }
    }
}
