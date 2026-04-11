pub struct Sample {
    pub label: &'static str,
    pub payload: &'static str,
}

pub const SAMPLES: &[Sample] = &[
    Sample {
        label: "JWT",
        payload: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkFkYSBMb3ZlbGFjZSIsImlhdCI6MTUxNjIzOTAyMn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
    },
    Sample {
        label: "PNG data URI",
        payload: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Zn5sUQAAAAASUVORK5CYII=",
    },
    Sample {
        label: "JSON",
        payload: "eyJ1c2VyIjoiYWRhIiwicm9sZSI6ImFkbWluIiwiYWN0aXZlIjp0cnVlLCJ0YWdzIjpbInJ1c3QiLCJnYW1lZGV2Il19",
    },
];
