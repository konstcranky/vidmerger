mod integration {
    use assert_cmd::{assert::Assert, Command};
    use k9::assertions::{greater_than::assert_greater_than, lesser_than::assert_lesser_than};
    use regex::Regex;
    use std::fs;
    use std::fs::File;
    use stdext::function_name;

    static BIN: &'static str = "vidmerger";

    #[cfg(test)]
    #[ctor::ctor]
    fn prepare() {
        use std::fs::{self, File};

        println!("👷 Doing preparations...");

        fs::remove_dir_all("data").unwrap_or_default();
        fs::create_dir_all("data").unwrap();

        download(
            "https://vidmerger.s3.eu-central-1.amazonaws.com/cat-1.mp4",
            "data/1.mp4",
        );
        download(
            "https://vidmerger.s3.eu-central-1.amazonaws.com/cat-2.mp4",
            "data/2.mp4",
        );
        File::create("data/.3.mp4").unwrap();

        download(
            "https://vidmerger.s3.eu-central-1.amazonaws.com/cat-1.mp3",
            "data/4.mp3",
        );
        download(
            "https://vidmerger.s3.eu-central-1.amazonaws.com/cat-2.mp3",
            "data/5.mp3",
        );

        println!("✅ Preparations done!");
    }

    #[test]
    fn call_merger() {
        let test_name = function_name!().split("::").last().unwrap();
        prep(test_name);

        let res = get_output(
            Command::cargo_bin(BIN)
                .unwrap()
                .arg(format!("data/{}", test_name))
                .assert()
                .success(),
        );

        assert!(res.contains("🐣 Generated"));
        check_for_merged_file(test_name, "output.mp4");
    }

    #[test]
    fn call_merger_on_audio_files() {
        let test_name = function_name!().split("::").last().unwrap();
        prep_audio(test_name);

        let res = get_output(
            Command::cargo_bin(BIN)
                .unwrap()
                .arg(format!("data/{}", test_name))
                .assert()
                .success(),
        );

        assert!(res.contains("🐣 Generated"));
        check_for_merged_file(test_name, "output.mp3");
    }

    #[test]
    fn call_merger_without_args() {
        let res = get_output_err(Command::cargo_bin(BIN).unwrap().assert().failure());

        assert!(Regex::new(r"vidmerger(\.exe)? \[OPTIONS] <TARGET_DIR>")
            .unwrap()
            .is_match(&res));
    }

    #[test]
    fn call_merger_and_fail_due_to_not_existing_directory() {
        let res = get_output_err(
            Command::cargo_bin(BIN)
                .unwrap()
                .arg(format!("data/nothing"))
                .assert()
                .failure(),
        );

        assert!(
            res.contains("No such file or directory")
                || res.contains("The system cannot find the path specified")
        );
    }

    #[test]
    fn call_merger_and_skip_hidden_vids() {
        let test_name = function_name!().split("::").last().unwrap();
        prep_with_hidden_file(test_name);

        let res = get_output(
            Command::cargo_bin(BIN)
                .unwrap()
                .arg("--verbose")
                .arg(format!("data/{}", test_name))
                .assert()
                .success(),
        );

        assert!(res.contains("🐣 Generated"));
        assert!(res.contains("1.mp4"));
        assert!(!res.contains(".3.mp4"));
        check_for_merged_file(test_name, "output.mp4");
    }

    #[test]
    fn call_merger_without_ffmpeg() {
        Command::cargo_bin(BIN)
            .unwrap()
            .arg("data")
            .env_clear()
            .assert()
            .failure()
            .stderr(format!(
                "❌ ffmpeg is not available. Please install it first.\n"
            ));
    }

    #[test]
    fn call_merger_against_mp4() {
        Command::cargo_bin(BIN)
            .unwrap()
            .args(&["--format", "mp4"])
            .arg("data")
            .assert()
            .success();
    }

    #[test]
    fn check_for_binaries() {
        if which::which("ffmpeg").is_err() {
            panic!("❌ ffmpeg wasn't found");
        }
    }

    #[test]
    fn call_merger_with_fps_changer() {
        let test_name = function_name!().split("::").last().unwrap();
        prep_with_different_fps_values(test_name);

        let res = get_output_err(
            Command::cargo_bin(BIN)
                .unwrap()
                .arg(format!("data/{}", test_name))
                .assert()
                .success(),
        );

        assert!(!res.contains("Non-monotonous DTS"));
        assert!(get_video_info(&format!("data/{}/output.mp4", test_name)).contains("28 fps"));
        check_for_merged_file(test_name, "output.mp4");
    }

    #[test]
    fn call_merger_with_fps_changer_with_fps_cli_arg() {
        let test_name = function_name!().split("::").last().unwrap();
        prep_with_different_fps_values(test_name);

        let res = get_output_err(
            Command::cargo_bin(BIN)
                .unwrap()
                .args(["--fps", "25"])
                .arg(format!("data/{}", test_name))
                .assert()
                .success(),
        );

        assert!(!res.contains("Non-monotonous DTS"));
        assert!(get_video_info(&format!("data/{}/output.mp4", test_name)).contains("25 fps"));
        check_for_merged_file(test_name, "output.mp4");
    }

    #[test]
    fn call_merger_without_fps_changer_on_vids_with_different_fps_values() {
        if cfg!(target_os = "linux") {
            std::process::exit(0)
        }
        let test_name = function_name!().split("::").last().unwrap();
        prep_with_different_fps_values(test_name);

        get_output_err(
            Command::cargo_bin(BIN)
                .unwrap()
                .arg(format!("data/{}", test_name))
                .arg("--skip-fps-changer")
                .assert()
                .success(),
        );

        assert!(get_video_info(&format!("data/{}/output.mp4", test_name)).contains("28 fps"));
        check_for_merged_file(test_name, "output.mp4");
    }

    // ----------------------------------------------------------------

    fn prep(test_name: &str) {
        fs::create_dir(format!("data/{}", test_name)).unwrap_or_default();
        fs::copy("data/1.mp4", format!("data/{}/1.mp4", test_name)).unwrap();
        fs::copy("data/2.mp4", format!("data/{}/2.mp4", test_name)).unwrap();
    }

    fn prep_with_hidden_file(test_name: &str) {
        prep(test_name);
        std::fs::File::create(format!("data/{}/.3.mp4", test_name)).unwrap();
    }

    fn prep_audio(test_name: &str) {
        fs::create_dir(format!("data/{}", test_name)).unwrap_or_default();
        fs::copy("data/4.mp3", format!("data/{}/4.mp3", test_name)).unwrap();
        fs::copy("data/5.mp3", format!("data/{}/5.mp3", test_name)).unwrap();
    }

    fn prep_with_different_fps_values(test_name: &str) {
        fs::create_dir(format!("data/{}", test_name)).unwrap_or_default();
        fs::copy("data/1.mp4", format!("data/{}/1.mp4", test_name)).unwrap();
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-i")
            .arg(format!("data/{}/1.mp4", test_name))
            .arg("-r")
            .arg("28")
            .arg(format!("data/{}/2.mp4", test_name))
            .output()
            .unwrap();
    }

    fn check_for_merged_file(test_name: &str, merged_file_name: &str) {
        let len = fs::metadata(format!("data/{}/{}", test_name, merged_file_name))
            .unwrap()
            .len();
        assert_greater_than(len, 600000);
        assert_lesser_than(len, 700000);
    }

    fn get_output(assert: Assert) -> String {
        String::from_utf8(assert.get_output().to_owned().stdout).unwrap()
    }

    fn get_output_err(assert: Assert) -> String {
        String::from_utf8(assert.get_output().to_owned().stderr).unwrap()
    }

    fn download(url: &str, out: &str) {
        let mut reader = ureq::get(url).call().unwrap().into_reader();
        let mut file = File::create(out).unwrap();
        std::io::copy(&mut reader, &mut file).unwrap();
    }

    fn get_video_info(file_path: &str) -> String {
        let output = Command::new("ffmpeg").arg("-i").arg(file_path).output();
        String::from_utf8_lossy(&output.unwrap().stderr).to_string()
    }
}
