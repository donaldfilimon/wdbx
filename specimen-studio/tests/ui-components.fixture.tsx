import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { useIsMobile } from '../hooks/use-mobile';
import {
  Carousel,
  CarouselContent,
  CarouselItem,
  CarouselNext,
  CarouselPrevious,
} from '../components/ui/carousel';
import {
  InputGroup,
  InputGroupAddon,
  InputGroupTextarea,
} from '../components/ui/input-group';

function Fixture() {
  const mobile = useIsMobile();
  const [show, setShow] = useState(true);
  return (
    <>
      <output data-testid="mobile">{String(mobile)}</output>
      <InputGroup>
        <InputGroupAddon data-testid="addon">
          Notes{' '}
          <a href="#help" data-testid="link">
            Help
          </a>
        </InputGroupAddon>
        <InputGroupTextarea aria-label="Notes" />
      </InputGroup>
      <button onClick={() => setShow(false)}>Unmount carousel</button>
      {show && (
        <Carousel aria-label="Examples">
          <CarouselContent>
            <CarouselItem>One</CarouselItem>
            <CarouselItem>Two</CarouselItem>
          </CarouselContent>
          <CarouselPrevious />
          <CarouselNext />
        </Carousel>
      )}
    </>
  );
}

createRoot(document.getElementById('root')!).render(<Fixture />);
