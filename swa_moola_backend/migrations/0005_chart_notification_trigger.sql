
CREATE OR REPLACE FUNCTION notify_conversation_participants()
RETURNS TRIGGER AS $$
DECLARE
    v_participant_id UUID;
BEGIN
    -- Loop through all participants in this conversation EXCEPT the sender
    FOR v_participant_id IN 
        SELECT user_id 
        FROM conversation_participants 
        WHERE conv_id = NEW.conv_id AND user_id != NEW.sender_id
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

-- 2. Bind the function to fire every time a new row hits the messages table
CREATE TRIGGER trg_after_message_insert
AFTER INSERT ON messages
FOR EACH ROW
EXECUTE FUNCTION notify_conversation_participants();