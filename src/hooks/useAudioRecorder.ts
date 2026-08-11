import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

export function useAudioRecorder(onTranscription: (text: string) => void) {
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);

  const startRecording = useCallback(async () => {
    try {
      await invoke('start_recording_cmd');
      setIsRecording(true);
    } catch (err) {
      console.error('Failed to start recording via Rust:', err);
      alert('Could not start the microphone via native backend.');
    }
  }, []);

  const stopRecording = useCallback(async () => {
    if (!isRecording) return;
    
    setIsRecording(false);
    setIsTranscribing(true);
    try {
      const text = await invoke<string>('stop_recording_and_transcribe_cmd');
      if (text.trim()) {
        onTranscription(text);
      }
    } catch (err) {
      console.error('Transcription failed:', err);
      alert('Failed to transcribe audio.');
    } finally {
      setIsTranscribing(false);
    }
  }, [isRecording, onTranscription]);

  const toggleRecording = useCallback(() => {
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
  }, [isRecording, startRecording, stopRecording]);

  return {
    isRecording,
    isTranscribing,
    toggleRecording,
  };
}
