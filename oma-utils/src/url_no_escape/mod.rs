/// Get no escape url
pub fn url_no_escape(s: &str) -> String {
    let mut tmp = s.to_string();
    let mut c = 0;
    loop {
        if c == 32 {
            panic!("loop > 32 {tmp}");
        }
        let res = url_escape::decode(&tmp);
        let res2 = url_escape::decode(&res);
        if res == res2 {
            return res.to_string();
        } else {
            tmp = res.to_string();
        }

        c += 1;
    }
}
