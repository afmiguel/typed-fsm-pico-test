stateDiagram-v2
    [*] --> LedOff

    note right of LedOff
        Entry: LED Low
    end note

    LedOff --> LedOn : TimerTick

    note right of LedOn
        Entry: LED High
        Entry: Trigger ADC
    end note

    LedOn --> LedOff : TimerTick
    LedOn --> HighValueWait : AdcResult(val) > 70

    note right of HighValueWait
        Entry: LED Low
        Entry: wait_ticks = 0
    end note

    HighValueWait --> LedOff : TimerTick (Timeout & Safe)
    HighValueWait --> HighValueWait : TimerTick (Inc wait & Trigger)
    HighValueWait --> HighValueWait : AdcResult (Update val)