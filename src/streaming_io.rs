use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use csv::{Reader, Writer};

pub struct StreamingReader {
    batch_size: usize,
}

pub struct StreamingWriter {
    file_path: String,
    batch_size: usize,
}

#[derive(Debug, Clone)]
pub struct DataRow {
    pub fields: Vec<String>,
}

impl StreamingReader {
    pub fn new(batch_size: usize) -> Self {
        StreamingReader { batch_size }
    }

    pub fn read_csv_batches<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<Vec<DataRow>>, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let mut csv_reader = Reader::from_reader(reader);
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();

        for result in csv_reader.records() {
            let rec = result.map_err(|e| e.to_string())?;
            let fields = rec.iter().map(|s| s.to_string()).collect();
            current_batch.push(DataRow { fields });

            if current_batch.len() >= self.batch_size {
                batches.push(current_batch);
                current_batch = Vec::new();
            }
        }

        if !current_batch.is_empty() {
            batches.push(current_batch);
        }

        Ok(batches)
    }

    pub fn read_json_batches<P: AsRef<Path>>(&self, path: P) -> Result<Vec<Vec<String>>, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();

        for res in reader.lines() {
            let line = res.map_err(|e| e.to_string())?;
            current_batch.push(line);

            if current_batch.len() >= self.batch_size {
                batches.push(current_batch);
                current_batch = Vec::new();
            }
        }

        if !current_batch.is_empty() {
            batches.push(current_batch);
        }

        Ok(batches)
    }
}

impl StreamingWriter {
    pub fn new(file_path: String, batch_size: usize) -> Self {
        StreamingWriter {
            file_path,
            batch_size,
        }
    }

    pub fn write_csv_batches(&self, batches: Vec<Vec<DataRow>>) -> Result<(), String> {
        let file = File::create(&self.file_path).map_err(|e| e.to_string())?;
        let mut writer = Writer::from_writer(file);

        for batch in batches {
            for row in batch {
                writer.write_record(&row.fields).map_err(|e| e.to_string())?;
            }
        }

        writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn write_json_lines(&self, lines: Vec<String>) -> Result<(), String> {
        let file = File::create(&self.file_path).map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(file);

        for line in lines {
            writeln!(writer, "{}", line).map_err(|e| e.to_string())?;
        }

        writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub struct StreamingStats {
    pub total_rows: usize,
    pub total_batches: usize,
    pub batch_size: usize,
    pub peak_memory_mb: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_reader_new() {
        let reader = StreamingReader::new(100);
        assert_eq!(reader.batch_size, 100);
    }

    #[test]
    fn test_streaming_writer_new() {
        let writer = StreamingWriter::new("output.csv".to_string(), 100);
        assert_eq!(writer.batch_size, 100);
    }

    #[test]
    fn test_data_row() {
        let row = DataRow {
            fields: vec!["a".to_string(), "b".to_string()],
        };
        assert_eq!(row.fields.len(), 2);
    }
}
