export function getBrowserIanaTimezone() {
    const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    console.log('JavaScript: getBrowserIanaTimezone called, returning:', timezone);
    return timezone;
}

export function normalizeIanaTimezone(tz) {
    if (!tz || typeof tz !== 'string') return 'UTC';
    const map = {
        'America/NewYork': 'America/New_York',
        'US/Eastern': 'America/New_York',
        'US/Central': 'America/Chicago',
        'US/Mountain': 'America/Denver',
        'US/Pacific': 'America/Los_Angeles',
        'GMT': 'UTC'
    };
    let normalized = map[tz] || tz;
    try {
        // Validate using Intl; this will throw on invalid tz
        new Intl.DateTimeFormat('en-US', { timeZone: normalized });
        return normalized;
    } catch (_) {
        console.warn('[tz] invalid IANA timezone, falling back to UTC:', tz);
        return 'UTC';
    }
}

export function getBrowserLocalTimezoneOffset() {
    const now = new Date();
    const offset = -now.getTimezoneOffset();
    const sign = offset >= 0 ? '+' : '-';
    const abs = Math.abs(offset);
    const hours = String(Math.floor(abs / 60)).padStart(2, '0');
    const minutes = String(abs % 60).padStart(2, '0');
    return `${sign}${hours}:${minutes}`;
}

export function getTimezoneOffsetForDate(tz, isoDate) {
    // isoDate can be a local datetime string without timezone (e.g. "2024-01-15T14:30:00")
    // We want the offset of that local time in the provided IANA timezone relative to UTC.
    try {
        tz = normalizeIanaTimezone(tz);
        // Parse the provided local datetime as if in the target timezone, using Intl to get UTC instant.
        // Construct a Date from the parts so we avoid timezone interpretation by the JS engine.
        const [datePart, timePart] = isoDate.split('T');
        const [y, m, d] = datePart.split('-').map(Number);
        const [hh, mm = '0', ss = '0'] = timePart ? timePart.split(':') : ['0','0','0'];

        // Build a Date corresponding to that wall-clock in the target tz by formatting to UTC
        const formatter = new Intl.DateTimeFormat('en-US', {
            timeZone: tz,
            year: 'numeric', month: '2-digit', day: '2-digit',
            hour: '2-digit', minute: '2-digit', second: '2-digit',
            hour12: false
        });
        const asDate = new Date(Date.UTC(y, (m - 1), d, Number(hh), Number(mm), Number(ss)));
        const parts = formatter.formatToParts(asDate);

        // Extract the UTC instant that corresponds to the target tz wall-clock
        const get = (type) => parts.find(p => p.type === type)?.value;
        const yy = Number(get('year'));
        const MM = Number(get('month')) - 1;
        const dd = Number(get('day'));
        const HH = Number(get('hour'));
        const mM = Number(get('minute'));
        const SS = Number(get('second'));
        const utcInstant = Date.UTC(yy, MM, dd, HH, mM, SS);

        // Now compare that instant to the same wall-clock interpreted as UTC
        const wallAsUtc = Date.UTC(y, (m - 1), d, Number(hh), Number(mm), Number(ss));
        const offsetMinutes = Math.round((utcInstant - wallAsUtc) / 60000);
        console.log('[tz]', { tz, isoDate, offsetMinutes });

        const sign = offsetMinutes >= 0 ? '+' : '-';
        const abs = Math.abs(offsetMinutes);
        const hours = String(Math.floor(abs / 60)).padStart(2, '0');
        const minutes = String(abs % 60).padStart(2, '0');
        return `${sign}${hours}:${minutes}`;
    } catch (e) {
        console.error("Error in getTimezoneOffsetForDate:", e);
        return '+00:00';
    }
}

// Compute offset for a specific UTC instant (RFC3339 like 2024-09-08T19:04:00Z or +00:00)
export function getTimezoneOffsetForInstant(tz, isoInstant) {
    try {
        tz = normalizeIanaTimezone(tz);
        const dt = new Date(isoInstant);
        const fmt = new Intl.DateTimeFormat('en-US', {
            timeZone: tz,
            year: 'numeric', month: '2-digit', day: '2-digit',
            hour: '2-digit', minute: '2-digit', second: '2-digit',
            hour12: false,
        });
        const parts = fmt.formatToParts(dt);
        const get = (type) => parts.find(p => p.type === type)?.value;
        const yy = Number(get('year'));
        const MM = Number(get('month')) - 1;
        const dd = Number(get('day'));
        const HH = Number(get('hour'));
        const mm = Number(get('minute'));
        const ss = Number(get('second'));
        // This constructs the UTC instant whose wall-clock equals the tz-local time
        const utcOfLocal = Date.UTC(yy, MM, dd, HH, mm, ss);
        const offsetMinutes = Math.round((utcOfLocal - dt.getTime()) / 60000);
        console.log('[tz-instant]', { tz, isoInstant, offsetMinutes });
        const sign = offsetMinutes >= 0 ? '+' : '-';
        const abs = Math.abs(offsetMinutes);
        const hours = String(Math.floor(abs / 60)).padStart(2, '0');
        const minutes = String(abs % 60).padStart(2, '0');
        return `${sign}${hours}:${minutes}`;
    } catch (e) {
        console.error('Error in getTimezoneOffsetForInstant:', e);
        return '+00:00';
    }
}

export function getLocalDateTimeString(date) {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    return `${year}-${month}-${day}T${hours}:${minutes}`;
} 