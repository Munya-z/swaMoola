CREATE OR REPLACE FUNCTION notify_conversation_participants()
RETURNS TRIGGER AS $$
DECLARE
    v_participant_id UUID;
    v_current_user_id UUID;
BEGIN
    -- 1. Extract the current user ID securely from the session context
    BEGIN
        v_current_user_id := NULLIF(current_setting('app.current_user_id', true), '')::uuid;
    EXCEPTION WHEN OTHERS THEN
        v_current_user_id := NULL;
    END;

    -- 2. Loop through all participants in this conversation EXCEPT the current session sender
    FOR v_participant_id IN 
        SELECT user_id 
        FROM conversation_participants 
        WHERE conv_id = NEW.conv_id 
          AND (v_current_user_id IS NULL OR user_id != v_current_user_id)
    LOOP
        -- Notify each distinct participant on their private socket pipeline
        PERFORM pg_notify(
            'user_updates_' || v_participant_id::text,
            json_build_object(
                'event', 'new_message',
                'conversation_id', NEW.conv_id::text
            )::text
        );
    END LOOP;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_after_message_insert
AFTER INSERT ON messages
FOR EACH ROW
EXECUTE FUNCTION notify_conversation_participants();