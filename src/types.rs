use std::path::Path;

pub type FsResult<T> = std::result::Result<T, String>;

#[derive(Debug, Clone)]
pub struct FileHashes {
	pub md5: String,
	pub crc32: String,
	pub sha1: String,
	pub sha256: String,
	pub sha3_256: String,
}

impl FileHashes {
	pub fn to_report(&self, path: &Path) -> String {
		format!(
			"File: {path}\nMD5     : {md5}\nCRC32   : {crc32}\nSHA1    : {sha1}\nSHA256  : {sha256}\nSHA3-256: {sha3}\n",
			path = path.display(),
			md5 = self.md5,
			crc32 = self.crc32,
			sha1 = self.sha1,
			sha256 = self.sha256,
			sha3 = self.sha3_256,
		)
	}
}
