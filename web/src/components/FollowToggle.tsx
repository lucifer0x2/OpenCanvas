interface Props {
  follow: boolean;
  change: (follow: boolean) => void;
}

/** "Follow agent": switch to the project an agent creates or changes. Off by default. */
export function FollowToggle({ follow, change }: Props) {
  return (
    <label
      className="connection"
      title="Switch to the project an agent creates or changes"
    >
      <input
        type="checkbox"
        checked={follow}
        onChange={(event) => change(event.target.checked)}
      />
      Follow agent
    </label>
  );
}
