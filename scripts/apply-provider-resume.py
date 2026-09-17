from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        return text
    if old not in text:
        raise SystemExit(f"marker missing: {label}")
    return text.replace(old, new, 1)


# 1) Backend: expose unresolved provider-unavailability events so work can resume after restart.
p = Path("src-tauri/src/provider_unavailability.rs")
s = p.read_text()
if "pub fn list_open_events(" not in s:
    marker = "pub fn affected_appointments(\n"
    insert = '''pub fn list_open_events(c: &Connection) -> Result<Vec<ProviderUnavailabilityEvent>, String> {
    let mut stmt = c
        .prepare(
            "SELECT id,doctor_id,unavailable_from,unavailable_to,reason_code,reason_note,created_at,resolved_at
             FROM provider_unavailability_events
             WHERE resolved_at IS NULL
             ORDER BY unavailable_from DESC,id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ProviderUnavailabilityEvent {
                id: r.get(0)?,
                doctor_id: r.get(1)?,
                unavailable_from: r.get(2)?,
                unavailable_to: r.get(3)?,
                reason_code: r.get(4)?,
                reason_note: r.get(5)?,
                created_at: r.get(6)?,
                resolved_at: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

'''
    if marker not in s:
        raise SystemExit("marker missing: provider affected_appointments")
    s = s.replace(marker, insert + marker, 1)
p.write_text(s)

# 2) Tauri runtime command + registration.
p = Path("src-tauri/src/lib.rs")
s = p.read_text()
if "fn provider_unavailability_list_open(" not in s:
    marker = "#[tauri::command]\nfn provider_unavailability_create("
    command = '''#[tauri::command]
fn provider_unavailability_list_open(
    db: tauri::State<Db>,
    actor_token: Option<String>,
) -> Result<Vec<provider_unavailability::ProviderUnavailabilityEvent>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::APPOINTMENT_READ)?;
    with_db(&db, provider_unavailability::list_open_events)
}
'''
    if marker not in s:
        raise SystemExit("marker missing: provider create command")
    s = s.replace(marker, command + marker, 1)
if "            provider_unavailability_list_open,\n" not in s:
    marker = "            provider_unavailability_create,\n"
    if marker not in s:
        raise SystemExit("marker missing: provider handler registration")
    s = s.replace(marker, "            provider_unavailability_list_open,\n" + marker, 1)
p.write_text(s)

# 3) Frontend API.
p = Path("src/api.ts")
s = p.read_text()
if "providerUnavailabilityOpen:" not in s:
    marker = "createProviderUnavailability:(input:ProviderUnavailabilityInput)=>authed<ProviderUnavailabilityEvent>('provider_unavailability_create',{input}),"
    if marker not in s:
        raise SystemExit("marker missing: provider frontend create API")
    s = s.replace(
        marker,
        "providerUnavailabilityOpen:()=>authed<ProviderUnavailabilityEvent[]>('provider_unavailability_list_open')," + marker,
        1,
    )
p.write_text(s)

# 4) UI: load and resume open incidents after application restart.
p = Path("src/ProviderUnavailabilityPage.tsx")
s = p.read_text()
if "const [openEvents, setOpenEvents]" not in s:
    s = replace_once(
        s,
        "  const [doctors, setDoctors] = useState<Doctor[]>([]);\n",
        "  const [doctors, setDoctors] = useState<Doctor[]>([]);\n  const [openEvents, setOpenEvents] = useState<ProviderUnavailabilityEvent[]>([]);\n",
        "open-events state",
    )
old_effect = '''  useEffect(() => {
    api.doctors().then(setDoctors).catch((e) => setError(String(e)));
  }, []);'''
new_effect = '''  useEffect(() => {
    Promise.all([api.doctors(), api.providerUnavailabilityOpen()])
      .then(([doctorRows, eventRows]) => {
        setDoctors(doctorRows);
        setOpenEvents(eventRows);
      })
      .catch((e) => setError(String(e)));
  }, []);'''
if "api.providerUnavailabilityOpen()" not in s:
    s = replace_once(s, old_effect, new_effect, "initial provider load")
if "async function resumeEvent(" not in s:
    marker = "  async function registerEvent(e: FormEvent) {"
    resume = '''  async function resumeEvent(item: ProviderUnavailabilityEvent) {
    if (busy) return;
    setBusy(true);
    setError('');
    setMessage('');
    try {
      setEvent(item);
      setDoctorId(String(item.doctorId));
      setFrom(localDateTime(item.unavailableFrom));
      setTo(localDateTime(item.unavailableTo));
      setReasonCode(item.reasonCode);
      setReasonNote(item.reasonNote || '');
      await refreshAffected(item.id);
      setMessage('تم استئناف حالة التعذّر المفتوحة وتحميل المواعيد التي ما زالت تحتاج معالجة.');
    } catch (e) {
      setEvent(null);
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

'''
    if marker not in s:
        raise SystemExit("marker missing: registerEvent")
    s = s.replace(marker, resume + marker, 1)
if "setOpenEvents((current) => [created" not in s:
    s = replace_once(
        s,
        "      setEvent(created);\n      await refreshAffected(created.id);",
        "      setEvent(created);\n      setOpenEvents((current) => [created, ...current.filter((item) => item.id !== created.id)]);\n      await refreshAffected(created.id);",
        "created open event",
    )
if "setOpenEvents(await api.providerUnavailabilityOpen());" not in s:
    s = replace_once(
        s,
        "      await api.resolveProviderUnavailability(event.id, resolutions);\n      await refreshAffected(event.id);",
        "      await api.resolveProviderUnavailability(event.id, resolutions);\n      await refreshAffected(event.id);\n      setOpenEvents(await api.providerUnavailabilityOpen());",
        "resolved open event refresh",
    )
if "void api.providerUnavailabilityOpen().then(setOpenEvents)" not in s:
    marker = "    setReasonNote('');\n  }"
    replacement = "    setReasonNote('');\n    void api.providerUnavailabilityOpen().then(setOpenEvents).catch((e) => setError(String(e)));\n  }"
    s = replace_once(s, marker, replacement, "reset open event refresh")
if "حالات تعذّر مفتوحة تحتاج استكمال" not in s:
    marker = '''      {message && <div className="notice" role="status">{message}</div>}

      {!event ? ('''
    replacement = '''      {message && <div className="notice" role="status">{message}</div>}

      {!event && openEvents.length > 0 && <div className="notice providerOpenEvents">
        <strong>حالات تعذّر مفتوحة تحتاج استكمال</strong>
        <div className="providerOpenEventList">
          {openEvents.map((item) => <button type="button" key={item.id} disabled={busy} onClick={() => void resumeEvent(item)}>
            استئناف {doctors.find((doctor) => doctor.id === item.doctorId)?.name || `المعالج #${item.doctorId}`} — {displayDateTime(item.unavailableFrom)}
          </button>)}
        </div>
      </div>}

      {!event ? ('''
    s = replace_once(s, marker, replacement, "open event resume UI")
old_display = '''const displayDateTime = (value: string) =>
  new Date(value).toLocaleString('ar-SA-u-ca-gregory', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });'''
new_display = '''const displayDateTime = (value: string) => {
  const date = new Date(value);
  const text = date.toLocaleString('ar-SA-u-ca-gregory', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
  return `${text} • الشهر ${String(date.getMonth() + 1).padStart(2, '0')}`;
};'''
if "• الشهر ${String(date.getMonth() + 1).padStart(2, '0')}" not in s:
    s = replace_once(s, old_display, new_display, "full Gregorian display")
p.write_text(s)

# 5) Regression contract: permanently guard resume-after-restart behavior.
p = Path("scripts/ui-contract.mjs")
s = p.read_text()
if "provider unavailability open events can be resumed after restart" not in s:
    marker = "  ['provider unavailability backend checks working window replacement capacity and preserves classification'"
    extra = '''  ['provider unavailability open events can be resumed after restart', t.api.includes("providerUnavailabilityOpen:()=>authed<ProviderUnavailabilityEvent[]>('provider_unavailability_list_open')") && files.providerUnavailability.includes('حالات تعذّر مفتوحة تحتاج استكمال') && t.providerUnavailability.includes('resumeEvent(item)')],
  ['provider unavailability backend exposes authenticated open-event listing', t.providerRust.includes('pubfnlist_open_events') && files.runtime.includes('provider_unavailability_list_open')],
'''
    if marker not in s:
        raise SystemExit("marker missing: provider backend contract")
    s = s.replace(marker, extra + marker, 1)
s = s.replace("if (checks.length !== 70) {", "if (checks.length !== 72) {")
s = s.replace("exactly 70 checks", "exactly 72 checks")
p.write_text(s)

print("provider resume patch applied")
