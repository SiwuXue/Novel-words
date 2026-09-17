use super::{CachedLicense, LicenseError};
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use ring::rand::{SecureRandom,SystemRandom};
use sha2::{Digest,Sha256};

pub struct Storage { pub root: PathBuf, pub device_id: String }
impl Storage {
    pub fn new(root:&Path,device_id:&str)->Result<Self,LicenseError> {
        let root=root.join("license");private_dir(&root)?;
        Ok(Self{root,device_id:device_id.into()})
    }
    pub fn save(&self,cache:&CachedLicense)->Result<(),LicenseError> {
        let data=serde_json::to_vec(cache).map_err(|_|error())?;
        let encrypted=self.protect(&data)?;
        atomic_write(&self.root.join("license.cache"),&encrypted)
    }
    pub fn load(&self)->Result<Option<CachedLicense>,LicenseError> {
        let path=self.root.join("license.cache");
        let Some(bytes)=read_file_bounded(&path,32768)? else {return Ok(None);};
        let data=self.unprotect(&bytes)?;
        serde_json::from_slice(&data).map(Some).map_err(|_|error())
    }
    #[cfg(windows)]
    fn protect(&self,data:&[u8])->Result<Vec<u8>,LicenseError>{dpapi(data,&self.device_id,true)}
    #[cfg(windows)]
    fn unprotect(&self,data:&[u8])->Result<Vec<u8>,LicenseError>{dpapi(data,&self.device_id,false)}
    #[cfg(not(windows))]
    fn protect(&self,data:&[u8])->Result<Vec<u8>,LicenseError>{
        use ring::aead::{Aad,Nonce};
        let key=self.portable_key()?;let nonce=random::<12>()?;
        let mut encrypted=data.to_vec();key.seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce),Aad::from(self.device_id.as_bytes()),&mut encrypted).map_err(|_|error())?;
        Ok([b"NWL1".as_slice(),&nonce,&encrypted].concat())
    }
    #[cfg(not(windows))]
    fn unprotect(&self,data:&[u8])->Result<Vec<u8>,LicenseError>{
        use ring::aead::{Aad,Nonce};
        if data.len()<32||&data[..4]!=b"NWL1"{return Err(error());}
        let key=self.portable_key()?;let nonce=data[4..16].try_into().map_err(|_|error())?;let mut encrypted=data[16..].to_vec();
        key.open_in_place(Nonce::assume_unique_for_key(nonce),Aad::from(self.device_id.as_bytes()),&mut encrypted).map(|plain|plain.to_vec()).map_err(|_|error())
    }
    #[cfg(not(windows))]
    fn portable_key(&self)->Result<ring::aead::LessSafeKey,LicenseError>{
        let path=self.root.join("cache.key");
        let bytes=create_if_missing(&path,32,||Ok(random::<32>()?.to_vec()))?;
        Ok(ring::aead::LessSafeKey::new(ring::aead::UnboundKey::new(&ring::aead::AES_256_GCM,&bytes).map_err(|_|error())?))
    }
}

fn error()->LicenseError{LicenseError::new("CACHE_ERROR")}
fn read_bounded(reader: impl Read, limit: usize) -> Result<Vec<u8>, LicenseError> {
    let mut bytes = Vec::new();
    let cap = u64::try_from(limit.checked_add(1).ok_or_else(error)?).map_err(|_| error())?;
    reader.take(cap).read_to_end(&mut bytes).map_err(|_| error())?;
    if bytes.len() > limit { return Err(error()); }
    Ok(bytes)
}
fn read_file_bounded(path: &Path, limit: usize) -> Result<Option<Vec<u8>>, LicenseError> {
    match std::fs::File::open(path) {
        Ok(file) => read_bounded(file, limit).map(Some),
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(error()),
    }
}
fn read_private_file(path: &Path, length: usize) -> Result<Option<Vec<u8>>, LicenseError> {
    let bytes = read_file_bounded(path, length)?;
    if bytes.as_ref().is_some_and(|bytes| bytes.len() != length) { return Err(error()); }
    Ok(bytes)
}
fn create_if_missing(path: &Path, length: usize, generate: impl FnOnce() -> Result<Vec<u8>, LicenseError>) -> Result<Vec<u8>, LicenseError> {
    if let Some(bytes) = read_private_file(path, length)? { return Ok(bytes); }
    let bytes = generate()?;
    if bytes.len() != length { return Err(error()); }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(&bytes).map_err(|_| error())?;
            file.sync_all().map_err(|_| error())?;
            Ok(bytes)
        },
        // Another instance established the key/identity after our initial read.
        // Never replace it, including when that creator left a damaged file.
        Err(failure) if failure.kind() == std::io::ErrorKind::AlreadyExists => read_private_file(path, length)?.ok_or_else(error),
        Err(_) => Err(error()),
    }
}
fn private_dir(path:&Path)->Result<(),LicenseError>{
    std::fs::create_dir_all(path).map_err(|_|error())?;
    #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;std::fs::set_permissions(path,std::fs::Permissions::from_mode(0o700)).map_err(|_|error())?;}
    Ok(())
}
fn random<const N:usize>()->Result<[u8;N],LicenseError>{let mut bytes=[0;N];SystemRandom::new().fill(&mut bytes).map_err(|_|error())?;Ok(bytes)}
fn hex(bytes:&[u8])->String{bytes.iter().map(|b|format!("{b:02x}")).collect()}
fn atomic_write(path:&Path,bytes:&[u8])->Result<(),LicenseError>{
    let temporary=path.with_file_name(format!(".license-{}.tmp",hex(&random::<12>()?)));
    let result=(|| {
        let mut options=std::fs::OpenOptions::new();options.write(true).create_new(true);
        #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
        let mut file=options.open(&temporary).map_err(|_|error())?;
        file.write_all(bytes).map_err(|_|error())?;file.sync_all().map_err(|_|error())?;
        drop(file);std::fs::rename(&temporary,path).map_err(|_|error())
    })();
    if result.is_err(){let _=std::fs::remove_file(&temporary);}result
}

#[cfg(windows)]
fn dpapi(data:&[u8],device:&str,protect:bool)->Result<Vec<u8>,LicenseError>{
    use windows_sys::Win32::{Foundation::LocalFree,Security::Cryptography::{CRYPT_INTEGER_BLOB,CRYPTPROTECT_UI_FORBIDDEN,CryptProtectData,CryptUnprotectData}};
    let mut input=CRYPT_INTEGER_BLOB{cbData:u32::try_from(data.len()).map_err(|_|error())?,pbData:data.as_ptr() as *mut u8};
    let entropy=Sha256::digest(format!("NovelWords license:{device}"));
    let entropy_blob=CRYPT_INTEGER_BLOB{cbData:32,pbData:entropy.as_ptr() as *mut u8};
    let mut output=CRYPT_INTEGER_BLOB{cbData:0,pbData:std::ptr::null_mut()};
    // DPAPI uses the current Windows account and machine; no UI or server secrets.
    let ok=unsafe{if protect {CryptProtectData(&mut input,std::ptr::null(),&entropy_blob,std::ptr::null(),std::ptr::null(),CRYPTPROTECT_UI_FORBIDDEN,&mut output)}else{CryptUnprotectData(&mut input,std::ptr::null_mut(),&entropy_blob,std::ptr::null(),std::ptr::null(),CRYPTPROTECT_UI_FORBIDDEN,&mut output)}};
    if ok==0{return Err(error());}
    let bytes=unsafe{std::slice::from_raw_parts(output.pbData,output.cbData as usize).to_vec()};
    unsafe{LocalFree(output.pbData as *mut core::ffi::c_void);};Ok(bytes)
}

pub fn stable_device_id(app_data:&Path,product:&str)->Result<String,LicenseError>{
    #[cfg(windows)]
    if let Some(guid)=machine_guid(){return Ok(hex(&Sha256::digest(format!("PrismKey:{product}:{}",guid.to_ascii_lowercase()))));}
    #[cfg(target_os="linux")]
    for path in ["/etc/machine-id","/var/lib/dbus/machine-id"] {
        if let Ok(id)=std::fs::read_to_string(path){if id.trim().len()>=16{return Ok(hex(&Sha256::digest(format!("PrismKey:{product}:{}",id.trim()))));}}
    }
    let root=app_data.join("license");private_dir(&root)?;let path=root.join("device.id");
    let bytes=create_if_missing(&path,64,||Ok(hex(&random::<32>()?).into_bytes()))?;
    let id=std::str::from_utf8(&bytes).map_err(|_|error())?;
    if !id.chars().all(|c|c.is_ascii_hexdigit()) {return Err(error());}
    Ok(hex(&Sha256::digest(format!("PrismKey:{product}:{id}"))))
}
#[cfg(windows)]
fn machine_guid()->Option<String>{
    use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE,RegGetValueW,RRF_RT_REG_SZ,RRF_SUBKEY_WOW6464KEY};
    let key="SOFTWARE\\Microsoft\\Cryptography\0".encode_utf16().collect::<Vec<_>>();let value="MachineGuid\0".encode_utf16().collect::<Vec<_>>();let mut buffer=[0u16;128];let mut bytes=256u32;
    let result=unsafe{RegGetValueW(HKEY_LOCAL_MACHINE,key.as_ptr(),value.as_ptr(),RRF_RT_REG_SZ|RRF_SUBKEY_WOW6464KEY,std::ptr::null_mut(),buffer.as_mut_ptr() as *mut _,&mut bytes)};
    if result!=0{return None;}let length=buffer.iter().position(|n|*n==0)?;let guid=String::from_utf16(&buffer[..length]).ok()?;
    (guid.len()>=16).then_some(guid)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_cache_reads_only_the_limit_plus_one_byte() {
        struct CountingReader { read: usize }
        impl Read for CountingReader {
            fn read(&mut self, destination: &mut [u8]) -> std::io::Result<usize> {
                let count = destination.len().min(1_000_000usize.saturating_sub(self.read));
                destination[..count].fill(1); self.read += count; Ok(count)
            }
        }
        let mut reader = CountingReader { read: 0 };
        assert_eq!(read_bounded(&mut reader, 32768).unwrap_err().code, "CACHE_ERROR");
        assert!(reader.read <= 32769, "read {} bytes before rejecting the oversized cache", reader.read);
        assert_eq!(read_bounded(&b"valid"[..], 5).unwrap(), b"valid");
        assert_eq!(read_bounded(&b""[..], 5).unwrap(), b"");
    }
    #[test]
    fn first_creation_keeps_the_winning_key_and_device_identity() {
        let dir = super::super::test_dir();
        for length in [32, 64] {
            let path = dir.join(format!("private-{length}"));
            let winning_path = path.clone();
            // The competing creator wins after our missing-file read and before
            // our final create. This interleaving is deterministic on Windows too.
            let value = create_if_missing(&path, length, || {
                std::thread::spawn(move || {
                    let mut writer = std::fs::OpenOptions::new().write(true).create_new(true).open(winning_path).unwrap();
                    writer.write_all(&vec![7; length]).unwrap(); writer.sync_all().unwrap();
                }).join().unwrap();
                Ok(vec![9; length])
            }).unwrap();
            assert_eq!(value, vec![7; length]);
            assert_eq!(std::fs::read(&path).unwrap(), vec![7; length]);
            assert_eq!(create_if_missing(&path, length, || panic!("existing identity must not generate a replacement")).unwrap(), value);
        }
    }
    #[test]
    fn private_files_require_exact_lengths_and_keep_corruption_intact() {
        let dir = super::super::test_dir();
        for length in [32, 64] {
            for size in [length - 1, length + 1] {
                let path = dir.join(format!("invalid-{length}-{size}"));
                std::fs::write(&path, vec![3; size]).unwrap();
                assert_eq!(create_if_missing(&path, length, || panic!("corrupt identity must not be replaced")).unwrap_err().code, "CACHE_ERROR");
                assert_eq!(std::fs::read(path).unwrap(), vec![3; size]);
            }
            let path = dir.join(format!("new-{length}"));
            assert_eq!(create_if_missing(&path, length, || Ok(vec![2; length])).unwrap(), vec![2; length]);
            assert_eq!(std::fs::read(path).unwrap(), vec![2; length]);
        }
    }
    #[test]
    fn encrypted_cache_roundtrips_separately_from_learning_database() {
        let dir=super::super::test_dir();
        let store=Storage::new(&dir,"test-device-0123456789").unwrap();
        let cache=CachedLicense{card_key:"CY-PRIVATE-EXAMPLE-CARD".into(),token:Some("signed-token".into()),server_time:1700000000,wall_at_verify:1700000000,observed_wall:1700000000,denial_code:None,plan:Some("30d".into()),expires_at:Some(1702592000),token_expires_at:Some(1700086400)};
        store.save(&cache).unwrap();
        let bytes=std::fs::read(store.root.join("license.cache")).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(&cache.card_key));
        assert!(!String::from_utf8_lossy(&bytes).contains("signed-token"));
        let loaded=Storage::new(&dir,"test-device-0123456789").unwrap().load().unwrap().unwrap();
        assert_eq!(loaded.card_key,cache.card_key);
        assert_eq!(loaded.token,cache.token);
        assert!(!dir.join("novel_words.db").exists());
        std::fs::write(store.root.join("license.cache"),b"corrupted").unwrap();
        assert_eq!(store.load().unwrap_err().code,"CACHE_ERROR");
    }
}
