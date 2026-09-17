# ClinicDesk Server

خدمة الشبكة المركزية لـ ClinicDesk. الجهاز الرئيسي فقط يملك PostgreSQL ومخزن المرفقات؛ أجهزة العملاء لا تفتح SQLite أو مجلد مشاركة SMB.

## التشغيل

اضبط المتغيرات التالية على الجهاز الرئيسي:

- `DATABASE_URL=postgresql://...`
- `CLINICDESK_SERVER_BIND=0.0.0.0:8443`
- `CLINICDESK_SERVER_TOKEN=<random 32+ chars>`
- `CLINICDESK_TLS_CERT=<path to PEM certificate>`
- `CLINICDESK_TLS_KEY=<path to PEM private key>`

ثم شغّل:

```bash
cargo run -p clinicdesk-server
```

## المسارات الحالية

- `GET /health` — فحص الخدمة واتصال PostgreSQL.
- `GET /api/v1/server/info` — معلومات الخادم، ويتطلب `Authorization: Bearer <token>`.

هذه البنية هي نواة الخدمة الفعلية وليست وثيقة تصميم. ما يزال نقل عمليات المرضى والمواعيد والمرفقات والمستخدمين من أوامر Tauri المحلية إلى API الشبكة مرحلة تنفيذية مستقلة، ولا يجوز وصف وضع الشبكة بأنه مكتمل قبل اكتمال تلك المسارات واختبارات 10 عملاء متزامنين.
