package vn.bizclaw.app.service

import android.content.ContentUris
import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.provider.CalendarContract
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.text.SimpleDateFormat
import java.util.*

/**
 * Calendar Integration for meeting scheduling.
 * 
 * Features:
 * 1. Read upcoming meetings from calendar
 * 2. Create reminders for action items
 * 3. Schedule follow-up meetings
 * 4. Add events to calendar
 * 
 * Requires calendar permission: android.permission.READ_CALENDAR, android.permission.WRITE_CALENDAR
 */
class CalendarIntegration(private val context: Context) {

    companion object {
        private const val TAG = "CalendarIntegration"
    }

    /**
     * Meeting event from calendar.
     */
    data class CalendarEvent(
        val id: Long,
        val title: String,
        val description: String?,
        val location: String?,
        val startTime: Long,
        val endTime: Long,
        val organizer: String?,
        val attendees: List<String>,
        val allDay: Boolean,
    )

    /**
     * Create a calendar event (e.g., for follow-up meeting).
     */
    data class CreateEventRequest(
        val title: String,
        val description: String = "",
        val location: String = "",
        val startTime: Long,  // millis since epoch
        val endTime: Long,    // millis since epoch
        val reminderMinutes: Int = 30,
        val attendeeEmails: List<String> = emptyList(),
    )

    /**
     * Get all calendars available on device.
     */
    fun getCalendars(): List<CalendarInfo> {
        val calendars = mutableListOf<CalendarInfo>()
        
        try {
            val projection = arrayOf(
                CalendarContract.Calendars._ID,
                CalendarContract.Calendars.CALENDAR_DISPLAY_NAME,
                CalendarContract.Calendars.ACCOUNT_NAME,
                CalendarContract.Calendars.ACCOUNT_TYPE,
                CalendarContract.Calendars.CALENDAR_COLOR,
                CalendarContract.Calendars.IS_PRIMARY,
            )

            context.contentResolver.query(
                CalendarContract.Calendars.CONTENT_URI,
                projection,
                null,
                null,
                null,
            )?.use { cursor ->
                val idIdx = cursor.getColumnIndex(CalendarContract.Calendars._ID)
                val nameIdx = cursor.getColumnIndex(CalendarContract.Calendars.CALENDAR_DISPLAY_NAME)
                val accountIdx = cursor.getColumnIndex(CalendarContract.Calendars.ACCOUNT_NAME)
                val typeIdx = cursor.getColumnIndex(CalendarContract.Calendars.ACCOUNT_TYPE)
                val colorIdx = cursor.getColumnIndex(CalendarContract.Calendars.CALENDAR_COLOR)
                val primaryIdx = cursor.getColumnIndex(CalendarContract.Calendars.IS_PRIMARY)

                while (cursor.moveToNext()) {
                    calendars.add(CalendarInfo(
                        id = cursor.getLong(idIdx),
                        displayName = cursor.getString(nameIdx) ?: "",
                        accountName = cursor.getString(accountIdx) ?: "",
                        accountType = cursor.getString(typeIdx) ?: "",
                        color = cursor.getInt(colorIdx),
                        isPrimary = cursor.getInt(primaryIdx) == 1,
                    ))
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error getting calendars: ${e.message}")
        }

        return calendars
    }

    /**
     * Get upcoming events from primary calendar.
     */
    suspend fun getUpcomingEvents(maxEvents: Int = 10): List<CalendarEvent> = withContext(Dispatchers.IO) {
        val events = mutableListOf<CalendarEvent>()
        
        try {
            val now = System.currentTimeMillis()
            val weekFromNow = now + (7 * 24 * 60 * 60 * 1000L)

            val projection = arrayOf(
                CalendarContract.Events._ID,
                CalendarContract.Events.TITLE,
                CalendarContract.Events.DESCRIPTION,
                CalendarContract.Events.EVENT_LOCATION,
                CalendarContract.Events.DTSTART,
                CalendarContract.Events.DTEND,
                CalendarContract.Events.ORGANIZER,
                CalendarContract.Events.ALL_DAY,
            )

            val selection = "${CalendarContract.Events.DTSTART} >= ? AND ${CalendarContract.Events.DTSTART} <= ?"
            val selectionArgs = arrayOf(now.toString(), weekFromNow.toString())
            val sortOrder = "${CalendarContract.Events.DTSTART} ASC LIMIT $maxEvents"

            context.contentResolver.query(
                CalendarContract.Events.CONTENT_URI,
                projection,
                selection,
                selectionArgs,
                sortOrder,
            )?.use { cursor ->
                val idIdx = cursor.getColumnIndex(CalendarContract.Events._ID)
                val titleIdx = cursor.getColumnIndex(CalendarContract.Events.TITLE)
                val descIdx = cursor.getColumnIndex(CalendarContract.Events.DESCRIPTION)
                val locIdx = cursor.getColumnIndex(CalendarContract.Events.EVENT_LOCATION)
                val startIdx = cursor.getColumnIndex(CalendarContract.Events.DTSTART)
                val endIdx = cursor.getColumnIndex(CalendarContract.Events.DTEND)
                val orgIdx = cursor.getColumnIndex(CalendarContract.Events.ORGANIZER)
                val allDayIdx = cursor.getColumnIndex(CalendarContract.Events.ALL_DAY)

                while (cursor.moveToNext()) {
                    val id = cursor.getLong(idIdx)
                    val attendees = getEventAttendees(id)
                    
                    events.add(CalendarEvent(
                        id = id,
                        title = cursor.getString(titleIdx) ?: "(No title)",
                        description = cursor.getString(descIdx),
                        location = cursor.getString(locIdx),
                        startTime = cursor.getLong(startIdx),
                        endTime = cursor.getLong(endIdx),
                        organizer = cursor.getString(orgIdx),
                        attendees = attendees,
                        allDay = cursor.getInt(allDayIdx) == 1,
                    ))
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error getting events: ${e.message}")
        }

        events
    }

    /**
     * Get attendees for an event.
     */
    private fun getEventAttendees(eventId: Long): List<String> {
        val attendees = mutableListOf<String>()
        
        try {
            val projection = arrayOf(
                CalendarContract.Attendees.ATTENDEE_NAME,
                CalendarContract.Attendees.ATTENDEE_EMAIL,
            )

            val selection = "${CalendarContract.Attendees.EVENT_ID} = ?"
            val selectionArgs = arrayOf(eventId.toString())

            context.contentResolver.query(
                CalendarContract.Attendees.CONTENT_URI,
                projection,
                selection,
                selectionArgs,
                null,
            )?.use { cursor ->
                val nameIdx = cursor.getColumnIndex(CalendarContract.Attendees.ATTENDEE_NAME)
                val emailIdx = cursor.getColumnIndex(CalendarContract.Attendees.ATTENDEE_EMAIL)

                while (cursor.moveToNext()) {
                    val name = cursor.getString(nameIdx)
                    val email = cursor.getString(emailIdx)
                    attendees.add(name ?: email ?: "Unknown")
                }
            }
        } catch (_: Exception) { }

        return attendees
    }

    /**
     * Create a new calendar event.
     * Returns the event URI or null on failure.
     */
    suspend fun createEvent(request: CreateEventRequest): Long? = withContext(Dispatchers.IO) {
        try {
            val calendarId = getPrimaryCalendarId() ?: return@withContext null

            val values = ContentValues().apply {
                put(CalendarContract.Events.CALENDAR_ID, calendarId)
                put(CalendarContract.Events.TITLE, request.title)
                put(CalendarContract.Events.DESCRIPTION, request.description)
                put(CalendarContract.Events.EVENT_LOCATION, request.location)
                put(CalendarContract.Events.DTSTART, request.startTime)
                put(CalendarContract.Events.DTEND, request.endTime)
                put(CalendarContract.Events.EVENT_TIMEZONE, TimeZone.getDefault().id)
                put(CalendarContract.Events.HAS_ALARM, 1)
            }

            val uri = context.contentResolver.insert(CalendarContract.Events.CONTENT_URI, values)
            val eventId = uri?.lastPathSegment?.toLongOrNull()

            if (eventId != null && request.reminderMinutes > 0) {
                addReminder(eventId, request.reminderMinutes)
            }

            Log.i(TAG, "Created event: $eventId")
            eventId
        } catch (e: Exception) {
            Log.e(TAG, "Error creating event: ${e.message}")
            null
        }
    }

    /**
     * Create reminder for an action item.
     */
    suspend fun createActionReminder(
        task: String,
        assignee: String?,
        deadline: String?,
        meetingTitle: String,
    ): Long? {
        val deadlineMs = parseDeadline(deadline) ?: (System.currentTimeMillis() + 24 * 60 * 60 * 1000L)
        val endMs = deadlineMs + 30 * 60 * 1000L // 30 min default

        return createEvent(CreateEventRequest(
            title = "[BizClaw] Action: $task",
            description = "From meeting: $meetingTitle\nAssignee: ${assignee ?: "Unassigned"}\nTask: $task",
            startTime = deadlineMs,
            endTime = endMs,
            reminderMinutes = 30,
        ))
    }

    /**
     * Add reminder to an event.
     */
    private fun addReminder(eventId: Long, minutesBefore: Int) {
        try {
            val values = ContentValues().apply {
                put(CalendarContract.Reminders.EVENT_ID, eventId)
                put(CalendarContract.Reminders.MINUTES, minutesBefore)
                put(CalendarContract.Reminders.METHOD, CalendarContract.Reminders.METHOD_ALERT)
            }
            context.contentResolver.insert(CalendarContract.Reminders.CONTENT_URI, values)
        } catch (_: Exception) { }
    }

    /**
     * Get primary calendar ID.
     */
    private fun getPrimaryCalendarId(): Long? {
        try {
            val projection = arrayOf(CalendarContract.Calendars._ID)
            val selection = "${CalendarContract.Calendars.IS_PRIMARY} = ?"
            val selectionArgs = arrayOf("1")

            context.contentResolver.query(
                CalendarContract.Calendars.CONTENT_URI,
                projection,
                selection,
                selectionArgs,
                null,
            )?.use { cursor ->
                if (cursor.moveToFirst()) {
                    return cursor.getLong(0)
                }
            }

            // Fallback: return first calendar
            context.contentResolver.query(
                CalendarContract.Calendars.CONTENT_URI,
                projection,
                null,
                null,
                null,
            )?.use { cursor ->
                if (cursor.moveToFirst()) {
                    return cursor.getLong(0)
                }
            }
        } catch (_: Exception) { }
        
        return null
    }

    /**
     * Parse deadline string to milliseconds.
     * Supports formats: "dd/MM/yyyy", "yyyy-MM-dd", "dd/MM"
     */
    private fun parseDeadline(deadline: String?): Long? {
        if (deadline == null) return null
        
        val formats = listOf(
            SimpleDateFormat("dd/MM/yyyy HH:mm", Locale.getDefault()),
            SimpleDateFormat("dd/MM/yyyy", Locale.getDefault()),
            SimpleDateFormat("yyyy-MM-dd", Locale.getDefault()),
            SimpleDateFormat("dd/MM", Locale.getDefault()),
        )

        for (format in formats) {
            try {
                val date = format.parse(deadline)
                if (date != null) {
                    // If year not in string, use current year
                    if (!deadline.contains("20")) {
                        val calendar = Calendar.getInstance()
                        calendar.time = date
                        val currentYear = Calendar.getInstance().get(Calendar.YEAR)
                        calendar.set(Calendar.YEAR, currentYear)
                        return calendar.timeInMillis
                    }
                    return date.time
                }
            } catch (_: Exception) { }
        }
        
        return null
    }

    /**
     * Open calendar app to view event.
     */
    fun openCalendarApp() {
        try {
            val intent = Intent(Intent.ACTION_VIEW).apply {
                data = CalendarContract.Events.CONTENT_URI
                flags = Intent.FLAG_ACTIVITY_NEW_TASK
            }
            context.startActivity(intent)
        } catch (e: Exception) {
            Log.e(TAG, "Cannot open calendar: ${e.message}")
        }
    }

    /**
     * Open specific event in calendar app.
     */
    fun openEvent(eventId: Long) {
        try {
            val intent = Intent(Intent.ACTION_VIEW).apply {
                data = ContentUris.withAppendedId(CalendarContract.Events.CONTENT_URI, eventId)
                flags = Intent.FLAG_ACTIVITY_NEW_TASK
            }
            context.startActivity(intent)
        } catch (e: Exception) {
            Log.e(TAG, "Cannot open event: ${e.message}")
        }
    }
}

data class CalendarInfo(
    val id: Long,
    val displayName: String,
    val accountName: String,
    val accountType: String,
    val color: Int,
    val isPrimary: Boolean,
)
