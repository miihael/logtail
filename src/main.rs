use clap::{Arg, ArgAction, Command};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::process::exit;

fn main() {
    // Setting up command line argument parsing using clap
    let matches = Command::new("logcheck")
        .version("1.0")
        .author("Jonathan Middleton <jjm@ixtab.org.uk>, Paul Slootman <paul@debian.org>")
        .about("Processes log files and updates offsets")
        .arg(
            Arg::new("logfile")
                .short('f')
                .value_name("LOG_FILE")
                .help("Log file to read"),
        )
        .arg(
            Arg::new("offsetfile")
                .short('o')
                .value_name("OFFSET_FILE")
                .help("Offset file")
                .required(false),
        )
        .arg(
            Arg::new("testmode")
                .short('t')
                .action(ArgAction::SetTrue)
                .help("Runs in test mode (does not update the offset file)"),
        )
        .arg(
            Arg::new("files")
                .value_names(["LOG_FILE", "OFFSET_FILE"])
                .help("Log file and offset file"),
        )
        .get_matches();

    let mut logfile = matches.get_one::<String>("logfile").cloned();
    let mut offsetfile = matches.get_one::<String>("offsetfile").cloned();
    let test_mode = matches.get_flag("testmode");

    let files: Vec<String> = matches.get_many::<String>("files").unwrap_or_default().cloned().collect();

    if logfile.is_none() && files.is_empty() {
        eprintln!("No logfile to read. Use -f [LOGFILE].");
        exit(66);
    }

    if files.len() == 1 {
        logfile = Some(files[0].clone());
    } else if files.len() == 2 {
        logfile = Some(files[0].clone());
        offsetfile = Some(files[1].clone());
    }

    logfile = logfile.or_else(|| files.get(0).cloned());
    offsetfile = offsetfile.or_else(|| files.get(1).cloned());

    let logfile = match logfile {
        Some(f) => f,
        None => {
            eprintln!("No logfile to read. Use -f [LOGFILE].");
            exit(66);
        }
    };

    if !std::path::Path::new(&logfile).is_file() {
        eprintln!("File {} cannot be read.", logfile);
        exit(66);
    }

    let offsetfile = offsetfile.unwrap_or_else(|| format!("{}.offset", logfile));

    let log_file = match File::open(&logfile) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("File {} cannot be read.", logfile);
            exit(66);
        }
    };

    let mut inode = 0;
    let mut offset = 0;

    if let Ok(mut offset_file) = File::open(&offsetfile) {
        let reader = BufReader::new(&mut offset_file);
        for (i, line) in reader.lines().enumerate() {
            if let Ok(line) = line {
                if i == 0 {
                    inode = line.trim().parse().unwrap_or(0);
                } else if i == 1 {
                    offset = line.trim().parse().unwrap_or(0);
                }
            }
        }
    }
    
    let metadata = match std::fs::metadata(&logfile) {
        Ok(m) => m,
        Err(_) => {
            eprintln!("Cannot get {} file size.", logfile);
            exit(65);
        }
    };

    let size = metadata.len();
    let ino = metadata.ino();

    if inode == ino && offset as u64 == size {
        exit(0);
    }

    let mut reader = BufReader::new(log_file);

    if inode != ino || offset as u64 > size {
        offset = 0;
    }

    if inode == ino && offset > size as usize {
        println!("***************");
        println!("*** WARNING ***: Log file {} is smaller than last time checked!", logfile);
        println!("*************** This could indicate tampering.");
        offset = 0;
    }

    reader.seek(SeekFrom::Start(offset as u64)).unwrap();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let mut buffer = String::new();
    while let Ok(bytes_read) = reader.read_line(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        write!(handle, "{}", buffer).unwrap();
        buffer.clear();
    }

    let size = reader.seek(SeekFrom::Current(0)).unwrap();

    if !test_mode {
        let mut offset_file = match OpenOptions::new().write(true).create(true).truncate(true).open(&offsetfile) {
            Ok(f) => f,
            Err(_) => {
                eprintln!("File {} cannot be created. Check your permissions.", offsetfile);
                exit(73);
            }
        };
        writeln!(offset_file, "{}", ino).unwrap();
        writeln!(offset_file, "{}", size).unwrap();
    }
}
