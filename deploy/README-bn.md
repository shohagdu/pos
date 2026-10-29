# cPanel-এ POS আপলোড করার নিয়ম (ধাপে ধাপে)

ওয়েবসাইট: `https://jsaebike.com`

এই ফোল্ডারে যা আছে:

```
deploy/
├── README-bn.md        ← এই গাইড
├── pos-cpanel.zip      ← শুধু এই একটা ফাইল cPanel-এ আপলোড করবেন
└── pos-app/            ← zip-এর ভিতরে যা আছে (দেখার জন্য)
    ├── app.js          ← Node অ্যাপ (cPanel এটা চালাবে)
    ├── env.example     ← সেটিং ফাইলের নমুনা (নাম বদলে .env করবেন)
    ├── bin/
    │   ├── pos-api     ← মূল সার্ভার প্রোগ্রাম
    │   └── pos-worker  ← ব্যাকগ্রাউন্ড কাজের প্রোগ্রাম
    ├── public/         ← ওয়েবসাইটের পেজ (লগইন পেজ ইত্যাদি)
    ├── storage/        ← প্রোডাক্টের ছবি এখানে জমা হবে
    └── logs/           ← সমস্যা হলে লগ এখানে লেখা হবে
```

> নতুন প্যাকেজ বানাতে (ডেভেলপার মেশিনে): `infra/cpanel/build.sh`

---

## মূল কথা (সবচেয়ে গুরুত্বপূর্ণ)

cPanel-এর **Setup Node.js App**-এ যে **Application root** ফোল্ডার দেবেন,
সেই ফোল্ডারের ভিতরেই উপরের সব ফাইল (`app.js`, `.env`, `bin/`, `public/`) থাকতে হবে।

ওয়েবসাইটে যদি **"It works! NodeJS 20.20.2"** লেখা দেখায়, তার মানে:
cPanel নিজের একটা খালি নমুনা `app.js` চালাচ্ছে, আমাদের POS অ্যাপ নয়।
অর্থাৎ Application root ভুল ফোল্ডারে, অথবা ওই ফোল্ডারে আমাদের ফাইল নেই।
নিচের ধাপ ৪ ও ৫ ঠিকমতো করলে এটা ঠিক হয়ে যাবে।

---

## ধাপ ১: ডাটাবেস বানানো

cPanel → **MySQL® Databases**

1. **Create New Database** ঘরে লিখুন `pos` → **Create Database**।
   পুরো নাম হবে `আপনারইউজার_pos` (যেমন `jsaebike_pos`)। নামটা লিখে রাখুন।
2. নিচে **Add New User**: ইউজারনেম `posapp`, একটা শক্ত পাসওয়ার্ড দিন → **Create User**।
   পুরো নাম হবে `jsaebike_posapp`। নাম ও পাসওয়ার্ড লিখে রাখুন।
3. **Add User To Database**: ইউজার `jsaebike_posapp`, ডাটাবেস `jsaebike_pos` → **Add**।
   পরের পেজে **ALL PRIVILEGES** টিক দিয়ে → **Make Changes**।

> শর্ত: সার্ভারে **MySQL 8** লাগবে (MariaDB চলবে না)।
> cPanel হোম পেজের ডান পাশে "General Information"-এ ভার্সন দেখা যায়।

---

## ধাপ ২: পুরনো ফাইল সরানো

cPanel → **File Manager** → `public_html`

`public_html`-এর ভিতরে যদি `index.html`, `assets` বা অন্য পুরনো ফাইল থাকে,
সেগুলো একটা ব্যাকআপ ফোল্ডারে সরিয়ে দিন (যেমন হোম ফোল্ডারে `old_public_html`)।
না সরালে পুরনো পেজ দেখাবে, নতুন অ্যাপ নয়।

> `.htaccess` ফাইলটা সরাবেন না (cPanel নিজে এটা লেখে)।

---

## ধাপ ৩: পুরনো Node অ্যাপ মুছে ফেলা (একদম নতুন করে শুরু)

cPanel → **Setup Node.js App**

আগে যদি jsaebike.com-এর জন্য কোনো অ্যাপ বানানো থাকে:
**Stop App** → তারপর ডাস্টবিন (🗑 Destroy) আইকনে চাপ দিয়ে মুছে ফেলুন।

তারপর File Manager-এ হোম ফোল্ডারে (`/home/jsaebike/`) পুরনো `pos-app` ফোল্ডার থাকলে:

- ভিতরের `.env` ফাইলটা আগে ডাউনলোড করে রাখুন (আবার লাগবে), তারপর
- `pos-app` ফোল্ডারটা মুছে দিন।

---

## ধাপ ৪: zip আপলোড ও Extract

cPanel → **File Manager**

1. বাম পাশে একদম উপরে **Home** (🏠 `/home/jsaebike`)-এ যান।
   **`public_html`-এর ভিতরে নয়!**
2. উপরে **Upload** → `pos-cpanel.zip` ফাইলটা বেছে নিন → আপলোড শেষ হওয়া পর্যন্ত অপেক্ষা করুন।
3. File Manager-এ ফিরে `pos-cpanel.zip`-এ রাইট-ক্লিক → **Extract** → পাথ `/home/jsaebike` রেখে **Extract File(s)**।
4. এখন হোম ফোল্ডারে একটা নতুন ফোল্ডার আসবে: **`pos-app`**। এর ভিতরে ঢুকে দেখুন:

```
/home/jsaebike/pos-app/
   app.js   env.example   bin/   public/   storage/   logs/
```

5. `pos-cpanel.zip` এখন মুছে দিতে পারেন।

---

## ধাপ ৫: `.env` সেটিং ফাইল বানানো

1. File Manager → উপরে ডানে **Settings** → **Show Hidden Files (dotfiles)** টিক দিন → **Save**।
2. `pos-app` ফোল্ডারে `env.example`-এ রাইট-ক্লিক → **Rename** → নতুন নাম: `.env`
   (আগে থেকে ডাউনলোড করা পুরনো `.env` থাকলে সেটাই আপলোড করতে পারেন।)
3. `.env`-এ রাইট-ক্লিক → **Edit**। শুধু এই দুই লাইন বদলাবেন:

**ক) ডাটাবেস লাইন** — ধাপ ১-এর নাম ও পাসওয়ার্ড বসান:

```
DATABASE_URL=mysql://jsaebike_posapp:আপনার_পাসওয়ার্ড@localhost:3306/jsaebike_pos
```

পাসওয়ার্ডে বিশেষ চিহ্ন থাকলে এভাবে লিখতে হবে:

| পাসওয়ার্ডে আছে | লিখবেন |
|---|---|
| `@` | `%40` |
| `#` | `%23` |
| `:` | `%3A` |
| `/` | `%2F` |
| `?` | `%3F` |

উদাহরণ: পাসওয়ার্ড `Abc@123#` হলে লিখবেন `Abc%40123%23`।
সবচেয়ে সহজ: শুধু অক্ষর ও সংখ্যা দিয়ে পাসওয়ার্ড বানান।

**খ) গোপন চাবি (JWT_SECRET)** — যেকোনো লম্বা এলোমেলো লেখা (কমপক্ষে ৪০ অক্ষর), যেমন:

```
JWT_SECRET=k8Fq2LmZ7xRw4Np9Tb3Yc6Hd1Vs5Ge0Ja8Ku2Qo7Wi4Ef9
```

(এই উদাহরণটা হুবহু ব্যবহার করবেন না, নিজে কিছু অক্ষর বদলে দিন।)

4. উপরে ডানে **Save Changes** → বন্ধ করুন।

---

## ধাপ ৬: প্রোগ্রাম চালানোর অনুমতি (755)

1. `pos-app/bin` ফোল্ডারে ঢুকুন।
2. `pos-api`-এ রাইট-ক্লিক → **Change Permissions** → সংখ্যা ঘরে **`755`** লিখুন → **Change Permissions**।
3. `pos-worker`-এর জন্যও একই কাজ করুন।

---

## ধাপ ৭: Node.js অ্যাপ বানানো

cPanel → **Setup Node.js App** → **+ Create Application**

ঘরগুলো ঠিক এভাবে পূরণ করুন:

| ঘর | কী লিখবেন |
|---|---|
| Node.js version | 18 বা তার বেশি (যেমন 20.x) |
| Application mode | **Production** |
| Application root | **`pos-app`** (শুধু এটুকু, `/home/...` বা `public_html` নয়) |
| Application URL | `jsaebike.com` বেছে নিন, পাশের ঘর **খালি** রাখুন |
| Application startup file | **`app.js`** |

**Create** চাপুন। (**Run NPM Install** চাপার দরকার নেই।)

> সতর্কতা: Application root-এ লেখা ফোল্ডারে `app.js` না থাকলে cPanel নিজে
> একটা নমুনা `app.js` বানিয়ে দেয়, যেটা "It works!" দেখায়।
> তাই আগে ধাপ ৪ শেষ করে তারপর এই ধাপ করবেন।

তারপর **Restart** (বা **Stop App** → **Start App**) চাপুন।

---

## ধাপ ৮: পরীক্ষা করা

৩০ সেকেন্ড অপেক্ষা করে ব্রাউজারে খুলুন:

| লিংক | ঠিক থাকলে দেখাবে |
|---|---|
| `https://jsaebike.com/pos-status` | `OK: the API is running.` |
| `https://jsaebike.com/healthz` | `{"status":"ok"}` |
| `https://jsaebike.com/readyz` | `{"status":"ready"}` |
| `https://jsaebike.com/` | POS-এর লগইন পেজ |

প্রথমবার চালু হতে ১–২ মিনিট লাগতে পারে (ডাটাবেসের টেবিল তৈরি হয়)।

---

## সমস্যা হলে

### "It works! NodeJS 20.20.2" দেখাচ্ছে

cPanel-এর নমুনা অ্যাপ চলছে। চেক করুন:

1. Setup Node.js App → অ্যাপের পাশে ✏️ (Edit) → **Application root** কী লেখা?
2. File Manager-এ সেই ফোল্ডার খুলুন → `app.js`-এ রাইট-ক্লিক → **Edit**।
   - ভিতরে যদি ছোট্ট কোড আর `It works!` লেখা থাকে → এটা ভুল ফাইল।
     এটা মুছে দিন, ধাপ ৪ আবার করুন (অথবা Application root বদলে `pos-app` দিন)।
   - সঠিক `app.js` অনেক বড় (~৩৫০ লাইন), উপরে লেখা থাকে `cPanel "Setup Node.js App" launcher`।
3. **Stop App** → **Start App**।

### "API_STARTING" / "The server is starting" দেখাচ্ছে

অ্যাপ চলছে কিন্তু মূল সার্ভার (pos-api) চালু হতে পারছে না।
`https://jsaebike.com/pos-status` খুলুন — সেখানে কারণ ও সমাধান লেখা থাকবে।

| pos-status-এ লেখা | সমাধান |
|---|---|
| `Access denied for user` | ডাটাবেস ইউজার/পাসওয়ার্ড ভুল, অথবা ইউজারকে ডাটাবেসে Add করা হয়নি (ধাপ ১.৩, ধাপ ৫ক) |
| `Name does not resolve` / `failed to lookup address` | পাসওয়ার্ডে `@ # $` ইত্যাদি চিহ্ন আছে — শুধু অক্ষর ও সংখ্যা দিয়ে নতুন পাসওয়ার্ড দিন (ধাপ ৫ক) |
| `Unknown database` | ডাটাবেসের পুরো নাম দিন, যেমন `jsaebike_pos` |
| `Permission denied` | ধাপ ৬ (755) আবার করুন |
| `UUID_TO_BIN` বা `MariaDB` | সার্ভারে MySQL 8 নেই — হোস্টিং কোম্পানিকে MySQL 8 দিতে বলুন |
| `SUPER privilege` | হোস্টিং কোম্পানিকে বলুন: `log_bin_trust_function_creators=1` সেট করে দিতে |
| `missing required environment variable` | `.env`-এ কোনো লাইন নেই বা ভুল — ধাপ ৫ আবার দেখুন |
| `Not configured yet` | `.env`-এ DATABASE_URL এখনো বদলানো হয়নি (ধাপ ৫ক) |

আরও বিস্তারিত লগ: File Manager → `pos-app/logs/` → `pos-api.log`, `launcher.log`।

---

## পরে আপডেট দিতে হলে

1. ডেভেলপার মেশিনে `infra/cpanel/build.sh` চালিয়ে নতুন `deploy/pos-cpanel.zip` বানান।
2. Setup Node.js App → **Stop App**।
3. File Manager → হোম ফোল্ডারে zip আপলোড → **Extract** (পুরনো ফাইলের উপর লিখবে;
   `.env` আর `storage/` ঠিক থাকবে, কারণ zip-এ এগুলো নেই)।
4. ধাপ ৬ (755) আবার করুন।
5. **Start App**।

---

## জেনে রাখুন

- OTP কোড ও ইনভাইট লিংক এখন SMS-এ যায় না; `pos-app/logs/pos-api.log`-এ লেখা থাকে।
- প্রোডাক্টের ছবি থাকে `pos-app/storage/`-এ। ব্যাকআপে এই ফোল্ডার আর ডাটাবেস দুটোই রাখুন
  (cPanel → Backup)।
- অনেকক্ষণ কেউ না ঢুকলে হোস্টিং অ্যাপ ঘুম পাড়িয়ে দেয়; প্রথম রিকোয়েস্টে কয়েক সেকেন্ড
  `API_STARTING` দেখাতে পারে — এটা স্বাভাবিক।
