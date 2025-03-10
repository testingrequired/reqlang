import "./CodeCard.css";

type CodeCardProps = {
  children: string;
};

const CodeCard = (props: CodeCardProps) => {
  return <pre className="code-card">{props.children}</pre>;
};

export default CodeCard;
